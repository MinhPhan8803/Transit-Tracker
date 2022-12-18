use iced::{Application, Command, Settings, Element};
use iced::executor;
use iced::theme::Theme;
use iced::widget::{Button, Text, Column};
use std::net::{IpAddr, Ipv6Addr};
use maxminddb::{Reader, MaxMindDBError, geoip2};
use public_ip;
use tokio;
use tokio::runtime::Runtime;
fn main() -> iced::Result {
    BusTracker::run(Settings::default())
}

#[derive(Debug, Clone)]
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
        .join(" ");
        println!("{}", my_start_point);
        (
            Self {
                ip: my_ip,
                start_point: my_start_point,
                destination: " ".to_owned(),
                position: my_position,
                stops: Vec::new(),
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

        let rt = Runtime::new().unwrap();

        let new_ip: IpAddr = 
        rt.block_on(public_ip::addr())
        .unwrap_or(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));

        let cur_loc_button = Button::new("Update current location")
        .on_press(Message::NewIp(new_ip));

        let cur_loc_display = Text::new(&self.start_point);
        Column::new()
        .push(cur_loc_button)
        .push(cur_loc_display)
        .into()
    }
    fn theme(&self) -> Self::Theme {
        Theme::Light
    }
}

fn get_position(city_query: &Result<geoip2::City, MaxMindDBError>) -> Position {
    match city_query {
        Ok(x) => {
            match &x.location {
                Some(y) => Position { 
                    latitude: y.latitude.unwrap_or_default(), 
                    longitude: y.longitude.unwrap_or_default(), 
                },
                None => Position {
                    latitude: 0.0, longitude: 0.0
                }
            }
        },
        Err(_) => Position {
            latitude: 0.0, longitude: 0.0
        }
    }
}

fn get_starting_loc<'a>(city_query: &'a Result<geoip2::City, MaxMindDBError>) -> Vec<&'a str> {
    let mut loc: Vec<&'a str> = Vec::new();
    match city_query {
        Ok(x) => {
            match &x.city {
                Some(y) => {
                    if y.names.is_some() {
                        for name in y.names
                        .as_ref()
                        .unwrap()
                        .values() {
                            loc.push(name);
                        }
                    }
                },
                None => (),
            }
        },
        Err(_) => (),
    };
    match city_query {
        Ok(x) => {
            match &x.subdivisions {
                Some(y) => {
                    for subdivision in y {
                        if subdivision.names.is_some() {
                            for name in subdivision.names.as_ref().unwrap().values() {
                                loc.push(name);
                            }
                        }
                    }
                },
                None => (),
            }
        },
        Err(_) => (),
    };
    loc
}