use std::{
    fs,
    io::Write,
};
use serde::{
    Deserialize,
    Serialize,
};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Body {
    name: String,
    gravitational_mass: f64,
    radius: f64,
    position: [f64; 3],
    velocity: [f64; 3]
}
#[allow(dead_code)]
impl Body {
    pub fn new(
        name: String,
        gravitational_mass: f64,
        radius: f64,
        position: [f64; 3],
        velocity: [f64; 3]
    ) -> Body {
        Body {
            name,
            gravitational_mass,
            radius,
            position,
            velocity
        }
    }
    pub fn load_json(file_path: &str) -> Result<Body, String> {
        let buffer = match fs::read_to_string(file_path) {
            Err(x) => return Err(format!("Failed reading {}!\n{}", file_path, x)),
            x => x.unwrap()
        };
        match serde_json::from_str(&buffer) {
            Err(x) => Err(format!("Failed deserializing {}!\n{}", file_path, x)),
            Ok(x) => Ok(x)
        }
    }
    pub fn save_json(&self, file_path: &str) -> Result<(), String> {
        let creation = fs::File::create(file_path);
        let mut file = match creation {
            Err(x) => return Err(format!("Failed creating {}!\n{}", file_path, x)),
            x => x.unwrap()
        };
        let conversion = serde_json::to_string_pretty(&self);
        let contents = match conversion {
            Err(x) => return Err(
                format!("Failed serializing {}!\n{}",
                    file_path, x)),
            x => x.unwrap(),
        };
        match file.write_all(&contents.as_bytes()) {
            Err(x) => Err(format!("Failed writing to {}!\n{}", file_path, x)),
            _ => Ok(())
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn mass(&self) -> f64 {
        self.gravitational_mass
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn position(&self) -> [f64; 3] {
        self.position
    }
    pub fn velocity(&self) -> [f64; 3] {
        self.velocity
    }
    pub fn apply_acceleration(&mut self, delta_acceleration: Vec<f64>, delta_time: f64) {
        self.velocity.iter_mut()
            .zip(delta_acceleration.iter())
            .map(|(a, b)| {*a += b * delta_time}).collect()
    }
    pub fn update_position(&mut self, delta_time: f64) {
        self.position.iter_mut()
            .zip(self.velocity.iter())
            .map(|(a, b)| {*a += b * delta_time}).collect()
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use std::panic;
    use float_eq::assert_float_eq;
    fn test_save_and_load(body: &Body) -> Result<(), String> {
        let file_name = format!("save_test_{}", body.name);
        match body.save_json(&file_name) {
            Err(x) => {
                fs::remove_file(&file_name).unwrap();
                return Err(x)
            },
            _ => ()
        };
        let load = match Body::load_json(&file_name) {
            Err(x) => {
                fs::remove_file(&file_name).unwrap();
                return Err(format!("Failed Reloading load!\n{}", x))
            },
            x => x.unwrap()
        };
        assert_eq!(body.name, load.name);
        match panic::catch_unwind( || {
            assert_float_eq!(
                (
                    body.gravitational_mass,
                    body.radius,
                    body.position,
                    body.velocity
                ),
                (
                    load.gravitational_mass,
                    load.radius,
                    load.position,
                    load.velocity
                ),
                ulps <= (1, 1, [1,1,1], [1,1,1])
            )
        }) {
            Err(x) => Err(format!("Failed comparing {} variables!\n{:?}", file_name, x)),
            _ => {
                fs::remove_file(&file_name).unwrap();
                Ok(())
            }
        }
    }
    #[test]
    fn save_and_load_json() -> Result <(), String> {
        assert_eq!(
            test_save_and_load(
                &Body::new(
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
            ),
            Ok(())
        );
        Ok(())
    }
}