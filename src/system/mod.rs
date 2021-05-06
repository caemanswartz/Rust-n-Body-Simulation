pub mod body;
pub mod horizons_cgi;
use std::{
    sync::{
        Arc,
        RwLock
    },
    fs,
    io::Write
};
use serde::{
    Deserialize,
    Serialize,
};
use body::Body;
#[derive(Debug,Serialize,Deserialize)]
pub struct System {
    object: Vec<Arc<RwLock<Body>>>
}
impl System {
    pub fn new() -> System {
        System {
            object: Vec::new()
        }
    }
    pub fn save_json(&self, file_path: &str) -> Result<(), String> {
        let mut file = match fs::File::create(file_path) {
            Err(x) => return Err(format!("Error creating {}!\n{}", file_path, x)),
            x => x.unwrap()
        };
        let conversion = serde_json::to_string_pretty(&self);
        let contents = match conversion {
            Err(x) => return Err(
                format!("Error serializing {}!\n{}",
                    file_path, x)),
            x => x.unwrap(),
        };
        match file.write_all(&contents.as_bytes()) {
            Err(x) => Err(format!("Error writing to {}!\n{}", file_path, x)),
            _ => Ok(())
        }
    }
    pub fn load_json(file_path: &str) -> Result<System, String> {
        let buffer = match fs::read_to_string(file_path) {
            Err(x) => return Err(format!("Error reading {}!\n{}", file_path, x)),
            x => x.unwrap()
        };
        match serde_json::from_str(&buffer) {
            Err(x) => Err(format!("Error deserializing {}!\n{}", file_path, x)),
            Ok(x) => Ok(x)
        }
    }
    pub fn fetch_from_horizons<T: AsRef<str>>(list: &[T], date: &time::Date) -> Result<System, String> {
        let mut system = System::new();
        horizons_cgi::fetch_target_bodies(list, date)
            .iter().for_each(|a| {
                system.add(a.clone())
            });
        Ok(system)
    }
    pub fn add(&mut self, body: Body) {
        self.object.push(Arc::new(RwLock::new(body)))
    }
    pub fn size(&self) -> usize {
        self.object.len()
    }
    fn exchange_gravitational_forces(&mut self, delta_time: f64) {
        use itertools::Itertools;
        use rayon::prelude::*;
        self.object.iter().combinations(2).collect::<Vec<_>>()
            .par_iter().for_each(|x| {
                let a = x[0].clone();
                let a_lock = a.read().unwrap();
                let a_mass = a_lock.mass();
                let a_position = a_lock.position();
                drop(a_lock);
                let b = x[1].clone();
                let b_lock = b.read().unwrap();
                let b_mass = b_lock.mass();
                let b_position = b_lock.position();
                drop(b_lock);
                let position_difference: Vec<f64> = a_position.iter()
                    .zip(b_position.iter())
                    .map(|(s, o)| {s - o}).collect();
                let inv_r3 = position_difference.iter().map(|a| {a.powf(2.0)}).sum::<f64>().powf(-1.5) * 2.22972471E-15;
                let mut a_lock = a.write().unwrap();
                a_lock.apply_acceleration(
                    position_difference.iter().map(|x| {inv_r3 * b_mass * x}).collect(),
                    delta_time
                );
                drop(a_lock);
                let mut b_lock = b.write().unwrap();
                b_lock.apply_acceleration(
                    position_difference.iter().map(|x| {inv_r3 * a_mass * x}).collect(),
                    delta_time
                );
                drop(b_lock)
            })
    }
    fn apply_individual_velocities(&mut self, delta_time: f64) {
        use rayon::iter::{
            ParallelIterator,
            IntoParallelRefMutIterator
        };
        self.object.par_iter_mut().for_each(|a| {
            a.clone().write().unwrap().update_position(delta_time)
        })
    }
    pub fn kick_drift_kick_step(&mut self, delta_time: f64) {
        self.exchange_gravitational_forces(delta_time/2.0);
        self.apply_individual_velocities(delta_time);
        self.exchange_gravitational_forces(delta_time/2.0);
    }
    pub fn object_from_index(&self, index: usize) -> Option<Arc<RwLock<Body>>> {
        if index < self.size() {
            Some(self.object[index].clone())
        } else {
            None
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use float_eq::assert_float_eq;
    fn compare(control: &Body, test: &Body) {
        println!("compare {} to {}", control.name(), test.name());
        println!("  name...");
        assert_eq!(control.name(), test.name());
        println!("  mass...");
        assert_eq!(control.mass(), test.mass());
        println!("  position...");
        assert_float_eq!(control.position(), test.position(), ulps <= [1,1,1]);
        println!("  velocity...");
        assert_float_eq!(control.velocity(), test.velocity(), ulps <= [1,1,1]);
    }
    #[test]
    fn save_and_load_json() -> Result <(), String> {
        let mut system = System::new();
        system.add(
            Body::new(
                "Sol".to_string(),
                1.9885e30,
                0.0046472586,
                [
                    0.004494340582683912,
                    0.0009104614297180857,
                    -0.0000609949004549505
                ],
                [
                    -4.728900304182371E-7,
                    5.597222756099664E-6,
                    -1.29597103647589E-8
                ]
            )
        );
        assert_eq!(system.save_json("test.json"),Ok(()));
        let test = match System::load_json("test.json") {
            Err(x) => {
                fs::remove_file("test.json").unwrap();
                return Err(format!("Failed to load \'test.json\'!\n{}", x))
            },
            x => x.unwrap()
        };
        let data = match system.object_from_index(0) {
            None => {
                fs::remove_file("test.json").unwrap();
                return Err(format!("Failed to find object in system data"))
            },
            x => x.unwrap()
        };
        fs::remove_file("test.json").unwrap();
        let load = match test.object_from_index(0) {
            None => return Err(format!("Failed to find object in file data")),
            x => x.unwrap()
        };
        compare(&data.read().unwrap(), &load.read().unwrap());
        Ok(())
    }
    #[test]
    pub fn fetch_inner_planets() -> Result<(), String> {
        let mut control = System::new();
        control.add(
            Body::new("(10)Sun".to_string(),
                132712440041.93938,
                1.408,
                [
                    0.004494340582683912,
                    0.0009104614297180857,
                    -0.00006099490045495054
                ],
                [
                    -4.728900304182371e-7,
                    5.597222756099664e-6,
                    -1.29597103647589e-8
                ]
            )
        );
        control.add(
            Body::new("(199)Mercury".to_string(),
                22031.86855,
                5.427,
                [
                    0.06070711234471207,
                    0.3026468028702081,
                    0.01941189349867229
                ],
                [
                    -0.03329788198291871,
                    0.006209252911230533,
                    0.003565339249484719
                ]
            )
        );
        control.add(
            Body::new("(299)Venus".to_string(),
                324858.592,
                5.204,
                [
                    0.7277190107771533,
                    -0.05797334573515864,
                    -0.04262367265271026
                ],
                [
                    0.001548125944714224,
                    0.02007264491630452,
                    0.0001831615503605528
                ]
            )
        );
        control.add(
            Body::new("(399)Earth".to_string(),
                398600.435436,
                5.51,
                    [
                        0.4133060075292528,
                        -0.9296817278172866,
                        -0.0001236944559827514
                    ],
                    [
                        0.01547590112466981,
                        0.006866255831713478,
                        9.70628931268737e-7
                    ]
            )
        );
        control.add(
            Body::new("(499)Mars".to_string(),
                42828.375214,
                3.933,
                [
                    0.1509005281414424,
                    -1.434525832388259,
                    -0.03372714180567889
                ],
                [
                    0.01445229001740577,
                    0.002626682942613087,
                    -0.0003013214316118337
                ]
            )
        );
        let list = vec!("10", "199", "299", "399", "499");
        let date = time::Date::try_from_ymd(1969,07,16).unwrap();
        let test = System::fetch_from_horizons(&list, &date).unwrap();
        test.object.iter().enumerate().for_each(|(i, a)| {
            compare(&a.read().unwrap(), &control.object_from_index(i).unwrap().read().unwrap());
        });
        Ok(())
    }
}