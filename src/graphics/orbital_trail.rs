use kiss3d::{
    nalgebra::Point3,
    window::Window
};
pub struct OrbitalTrail {
    point: Vec<Point3<f32>>
}
impl OrbitalTrail {
    pub fn new(position: Point3<f32>, size: usize) -> OrbitalTrail {
        OrbitalTrail {
            point: vec![position; size]
        }
    }
    pub fn update(&mut self, position: Point3<f32>) {
        self.point.remove(0);
        self.point.push(position)
    }
    pub fn draw(&self, window: &mut Window, color: Point3<f32>) {
        use itertools::*;
        let length = self.point.len();
        self.point.iter().dropping(1)
            .zip(self.point.iter().dropping_back(1))
            .enumerate()
            .for_each(|(i, (a, b))| {
                window.draw_line(&a, &b, &(color * i as f32 / length as f32))
            });
    }
    pub fn last(&self) -> Option<&Point3<f32>> {
        self.point.last()
    }
}