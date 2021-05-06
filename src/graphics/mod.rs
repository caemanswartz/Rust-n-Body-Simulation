mod orbital_trail;
use kiss3d::{
    camera::{
        ArcBall,
        Camera
    },
    nalgebra::{
        Point2,
        Point3,
        Translation3,
        Vector2
    },
    scene::SceneNode,
    text::Font,
    window::Window
};
use crate::{
    system::{
        body::Body,
        System,
    }
};
use orbital_trail::OrbitalTrail;
struct Graphic {
    name: String,
    model: kiss3d::scene::SceneNode,
    color: Point3<f32>,
    trail: OrbitalTrail
}
impl Graphic {
    pub fn new(body: &Body, trail_size: usize, color: Point3<f32>, anchor: &mut SceneNode) -> Graphic {
        let position = body.position();
        Graphic {
            name: body.name().to_string(),
            model:
                {
                    let mut model = anchor.add_sphere((body.radius() * 6.6846E-09) as f32);
                    model.set_local_translation(
                        Translation3::new(
                            position[0] as f32,
                            position[1] as f32,
                            position[2] as f32
                        )
                    );
                    model.set_color(color.x,color.y,color.z);
                    model
                },
            color,
            trail:
                OrbitalTrail::new(
                    Point3::new(
                        position[0] as f32,
                        position[1] as f32,
                        position[2] as f32
                    ),
                    trail_size
                )
        }
    }
    pub fn update(&mut self, body: &Body) {
        let position = body.position();
        self.trail.update(
            Point3::new(
                position[0] as f32,
                position[1] as f32,
                position[2] as f32
            )
        );
    }
    pub fn draw(&mut self, window: &mut Window, camera: &ArcBall) {
        let position: Point3<f32> = self.trail.last().expect("No position found!").clone();
        self.model.set_local_transformation(Translation3::new(position.x, position.y, position.z).into());
        self.trail.draw(window, self.color);
        let window_size = Vector2::new(window.size()[0] as f32, window.size()[1] as f32);
        let window_coordinate = camera.project(&position, &window_size);
        window.draw_text(
            &self.name,
            &Point2::new(
                2.0 * window_coordinate.x,
                2.0 * (window_size.y - window_coordinate.y)
            ),
            24.0,
            &Font::default(),
            &Point3::new(
                0.6,
                0.6,
                0.6
            )
        );
    }
    pub fn last(&self) -> Option<&Point3<f32>> {
        self.trail.last()
    }
}
pub struct Graphics {
    focus: Option<usize>,
    camera: ArcBall,
    object: Vec<Graphic>
}
impl Graphics {
    pub fn new(system: &System, eye: Point3<f32>, at: Point3<f32>, trail_size: usize, anchor: &mut SceneNode) -> Graphics {
        let mut object = Vec::new();
        (0..system.size()).into_iter().for_each(|a| {
            object.push(
                Graphic::new(
                    &system.object_from_index(a).expect(&format!("Failed to get object {}", a)).read().unwrap(),
                    trail_size,
                    Point3::new(
                        1.0,
                        1.0,
                        1.0
                    ),
                    anchor
                )
            );
        });
        Graphics {
            focus: None,
            camera: ArcBall::new_with_frustrum(std::f32::consts::PI / 4.0, 0.00001, 1024.0, eye, at),
            object
        }
    }
    pub fn camera(&mut self) -> &mut ArcBall {
        &mut self.camera
    }
    pub fn focus(&mut self, target: usize) {
        if target < self.object.len() {
            self.focus = Some(target);
        }
    }
    pub fn focus_next(&mut self) {
        match self.focus {
            Some(x) => if x < self.object.len() - 1 {
                self.focus = Some(x + 1)
            } else {
                self.focus = Some(0)
            },
            _ => ()
        }
    }
    pub fn focus_last(&mut self) {
        match self.focus {
            Some(x) => if x > 0 {
                self.focus = Some(x - 1)
            } else {
                self.focus = Some(self.object.len() - 1)
            },
            _ => ()
        }
    }
    pub fn unfocus(&mut self) {
        self.focus = None
    }
    pub fn update(&mut self, system: &System) {
        self.object.iter_mut().enumerate().for_each(|(i, a)| {
            a.update(&system.object_from_index(i).expect("Index out of bounds!").read().unwrap())
        })
    }
    pub fn draw(&mut self, window: &mut Window) {
        let camera = &self.camera;
        self.object.iter_mut().for_each(|a| {
            a.draw(window, &camera);
        });
        match self.focus {
            Some(x) => {
                let new_at = self.object[x].last().unwrap().clone();
                self.camera.set_at(new_at);
            },
            _ => ()
        }
    }
}