use kiss3d::{
    event::{
        Action,
        Key,
        WindowEvent
    },
    light::Light,
    nalgebra::Point3,
    window::Window
};
use nbodysim::{
    system::System,
    graphics::Graphics
};
use std::env;
fn main() {
    let mut flags: Vec<Option<String>> = vec!(None; 4);
    let args: Vec<String> = env::args().collect();
    let flag = "get".to_string();
    let mut list = Vec::new();
    let mut i: usize = 0;
    for arg in args {
        if flags[0] == Some(flag.clone()) {
            flags[0] = Some(arg)
        } else if flags[1] == Some(flag.clone()) {
            flags[1] = Some(arg)
        } else if flags[2] == Some(flag.clone()) {
            flags[2] = Some(arg);
            flags[3] = Some(flag.clone())
        } else if flags[3] == Some(flag.clone()) {
            flags[3] = Some(arg.clone());
            i = match arg.parse::<usize>() {
                Ok(x) => x.clone(),
                Err(x) => {
                    println!("Failed to parse {}!\n{}", arg, x);
                    return
                }
            }
        } else if i > 0 {
            i = i -1;
            list.push(arg);
        } else {
            match arg.as_str() {
                "-L" => flags[0] = Some(flag.clone()),
                "-S" => flags[1] = Some(flag.clone()),
                "-F" => flags[2] = Some(flag.clone()),
                _ => ()
            }
        }
    }
    let delta_time = 1.0/24.0;
    let mut system = if flags[2] != None {
        let data = match &flags[2] {
            Some(x) => x.as_str(),
            _ => ""
        };
        let date = match time::Date::parse(data, "%F") {
            Ok(x) => x,
            Err(x) => {
                println!("Failed to parse date {}!\n{}", data, x);
                return
            }
        };
        match System::fetch_from_horizons(list.as_slice(), &date) {
            Ok(x) => x,
            Err(x) => {
                println!("Failed to fetch system data!\n{}", x.clone());
                return
            }
        }
    } else {
        let name = match &flags[0] {
            Some(x) => x,
            _ => "a_few_satellites_more_1969_07_16.json"
        };
        match System::load_json(&name) {
            Err(x) => {
                println!("Failed to load {}, fetching from HORIZONS...\n{}",name, x);
                let list = vec!(r"10", r"199", r"299", r"399", r"499", r"599", r"699", r"799", r"899", r"999",
                    r"301", r"401", r"402",
                    r"A801 AA", r"A807 FA", r"A802 FA", r"A849 GA", r"A854 RA", r"A910 TC", r"A903 KB", r"A904 HE", r"A851 OA", r"A804 RA", r"A852 FA", r"A858 CA",
                    r"501", r"502", r"503", r"504", r"505", r"506", r"507", r"508", r"509", r"510", r"511", r"512", r"513", r"514", r"515", r"516",
                    r"601", r"602", r"603", r"604", r"605", r"606", r"607", r"608", r"609", r"610", r"611", r"612", r"613", r"614", r"615", r"616", r"617", r"618",
                    r"701", r"702", r"703", r"704", r"705",
                    r"801", r"802", r"803", r"804", r"805", r"806", r"807", r"808", r"809", r"810", r"811", r"812", r"813", r"814",
                    r"901", r"902", r"903", r"904", r"905"
                );
                let date = time::Date::try_from_ymd(1969,07,16).unwrap();
                let system = System::fetch_from_horizons(&list, &date).unwrap();
                system.save_json(&name).unwrap();
                system
            }
            Ok(x) => x
        }
    };
    let mut window = Window::new("Kiss3d: solar system n-body simulator");
    window.set_light(Light::StickToCamera);
    let eye = Point3::new(0.0f32, 0.0, -1.0);
    let at =  Point3::origin();
    let mut graphics = Graphics::new(&system, eye, at, (40 as f64 / delta_time) as usize, &mut window.add_group());
    let camera = graphics.camera();
    camera.set_dist(1.0);
    camera.set_dist_step(0.5);
    camera.set_min_dist(0.0078125);
    camera.set_max_dist(7.0E2);
    while window.render_with_camera(graphics.camera()) {
        for event in window.events().iter() {
            match event.value {
                WindowEvent::Key(button, Action::Press, _) => {
                    match button {
                        Key::Key1 => graphics.focus(1),
                        Key::Key2 => graphics.focus(2),
                        Key::Key3 => graphics.focus(3),
                        Key::Key4 => graphics.focus(4),
                        Key::Key5 => graphics.focus(5),
                        Key::Key6 => graphics.focus(6),
                        Key::Key7 => graphics.focus(7),
                        Key::Key8 => graphics.focus(8),
                        Key::Key9 => graphics.focus(9),
                        Key::Key0 => graphics.focus(0),
                        Key::Equals => graphics.focus_next(),
                        Key::Minus => graphics.focus_last(),
                        Key::Back => graphics.unfocus(),
                        _ => ()
                    }
                },
                _ => ()
            }
        }
        system.kick_drift_kick_step(delta_time);
        graphics.update(&system);
        graphics.draw(&mut window);
    };
    match &flags[1] {
        Some(x) => match system.save_json(&x) {
            Ok(_) => (),
            Err(y) => {
                println!("Failed to save {}!\n{}", x, y)
            }
        },
        _ => ()
    };
}