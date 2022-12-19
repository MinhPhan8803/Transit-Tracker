use iced::{Application, Command, Settings, Element};
use iced::executor;
use iced::theme::Theme;
use iced::widget::{Button, Text, Column};
use std::fmt::Display;
use std::net::{IpAddr, Ipv6Addr};
use maxminddb::{Reader, MaxMindDBError, geoip2};
use public_ip;
use tokio::runtime::Runtime;
use tokio;
use reqwest::Client;
use serde::Deserialize;
use dotenvy;
fn main() -> iced::Result {
    BusTracker::run(Settings::default())
}

#[derive(Debug, Clone, Default)]
struct Position {
    latitude: f64,
    longitude: f64,
}

struct BusTracker {
    ip: IpAddr,
    start_point: String,
    destination: String,
    position: Position,
    stops: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    NewIp(IpAddr),
    NewStart(String),
    NewPos(Position),
    NewDest(String),
    NewStops(Vec<String>),
}

impl Application for BusTracker {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();
    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let rt = Runtime::new().unwrap();
        let my_ip: IpAddr = 
        rt.block_on(public_ip::addr())
        .unwrap_or(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));

        println!("{}", my_ip.to_string());
        let reader: Reader<Vec<u8>> = 
        Reader::open_readfile("data/GeoLite2-City.mmdb")
        .unwrap_or_else(|msg| panic!("{}", msg.to_string()));

        let city_query: Result<geoip2::City, MaxMindDBError> = 
        reader.lookup(my_ip);

        let my_position: Position = get_position(&city_query);
        let my_start_point = get_starting_loc(&city_query)
        .join(", ");
        println!("{}", my_start_point);
        let my_stops = rt.block_on(get_stops(&my_position));
        (
            Self {
                ip: my_ip,
                start_point: my_start_point,
                destination: " ".to_owned(),
                position: my_position,
                stops: my_stops,
            },
            Command::none(),
        )
    }
    fn title(&self) -> String {
        "Bus Tracker".to_owned()
    }
    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::NewIp(x) => self.ip = x,
            Message::NewStart(x) => self.start_point = x,
            Message::NewPos(x) => self.position = x,
            Message::NewDest(x) => self.destination = x,
            Message::NewStops(x) => self.stops = x,
        }
        Command::none()
    }
    fn view(&self) -> Element<Message> {

        let rt: Runtime = Runtime::new().unwrap();
        let new_ip: IpAddr = 
        rt.block_on(public_ip::addr())
        .unwrap_or(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));

        let cur_loc_button = Button::new("Update current location")
        .on_press(Message::NewIp(new_ip));

        let cur_loc_display = Text::new(&self.start_point);
        let stops_display = self.stops
        .iter()
        .map(|stop| Text::new(stop.as_str()))
        .collect::<Vec<Text>>();
        let mut col = Column::new()
        .push(cur_loc_button)
        .push(cur_loc_display);
        for stop_display in stops_display {
            col = col.push(stop_display);
        }
        col.into()
    }
    fn theme(&self) -> Self::Theme {
        Theme::Light
    }
}

fn get_position(city_query: &Result<geoip2::City, MaxMindDBError>) -> Position {
    if let Ok(x) = city_query {
        if let Some(y) = &x.location {
            return Position { 
                latitude: y.latitude.unwrap_or_default(), 
                longitude: y.longitude.unwrap_or_default(), 
            };
        }
    }
    Position::default()
}

fn get_starting_loc<'a>(city_query: &'a Result<geoip2::City, MaxMindDBError>) -> Vec<&'a str> {
    let mut loc: Vec<&'a str> = Vec::new();
    match city_query {
        Ok(x) => {
            if let Some(y) = &x.city {
                if let Some(names) = &y.names { 
                    loc.push(names
                        .values()
                        .next()
                        .unwrap());
                }
            }
            if let Some(y) = &x.subdivisions {
                for subdivision in y {
                    if let Some(names) = &subdivision.names {
                        loc.push(names
                            .values()
                            .next()
                            .unwrap());
                    }
                }
            }
            if let Some(y) = &x.postal {
                if let Some(code) = y.code {
                    loc.push(code);
                }
            }
        },
        Err(_) => (),
    };
    loc
}

async fn get_stops(pos: &Position) -> Vec<String> {
    let Ok(client) = Client::builder().build() else {
        return Vec::new();
    };
    let api_key = dotenvy::var("CUMTD").unwrap();
    let Ok(res) = 
    client
    .get("https://developer.cumtd.com/api/v2.2/json/getstopsbylatlon")
    .query(&[("key", api_key.as_str()), ("lat", pos.latitude.to_string().as_str()), ("lon", pos.longitude.to_string().as_str()), ("count", "5")])
    .send().await
    else {
        return Vec::new();
    };
    let Ok(json) = res.json::<BusStopRes>().await else {
        return Vec::new();
    };
    json.stops
    .iter()
    .map(|stop| stop.to_string())
    .collect()
}

#[derive(Deserialize)]
struct BusStopRes {
    stops: Vec<BusStop>,
}

#[derive(Deserialize)]
struct BusStop {
    stop_name: String,
    code: String,
    distance: f64,
}

impl Display for BusStop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Name: {}, Code: {}, Distance: {}", self.stop_name, self.code, self.distance)
    }
}