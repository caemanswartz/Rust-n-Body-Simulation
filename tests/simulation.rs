extern crate nbodysim;
use ansi_term::Colour;
use nbodysim::system::{
   body::Body,
   System 
};
use float_eq::assert_float_eq;
fn approximate_compare(control_star: &Body, control_body: &Body, test_star: &Body, test_body: &Body) {
    println!("  Approximately comparing test {} to HORIZONS {}...", test_body.name(), control_body.name());
    let position_difference = control_body.position().iter()
        .zip(test_body.position().iter()).map(|(c, t)| {(c - t).powf(2.0)})
        .sum::<f64>().sqrt();
    let control_difference = control_star.position().iter()
        .zip(control_body.position().iter()).map(|(c, t)| {(c - t).powf(2.0)})
        .sum::<f64>().sqrt();
    let test_difference = test_star.position().iter()
        .zip(test_body.position().iter()).map(|(c, t)| {(c - t).powf(2.0)})
        .sum::<f64>().sqrt();
    println!("    World position difference of {}...", position_difference);
    match std::panic::catch_unwind(|| {assert_float_eq!(test_difference, control_difference, r2nd <= 2.0E-2f64)}) {
        Ok(_) => println!("      {}: Distance to sun within 1% of HORIZONS data", Colour::Green.bold().paint("Passed")),
        Err(_) => println!("      {}: Distance to sun check!", Colour::Red.bold().paint("Failed"))
    }
}
fn simulate_planets_for(days: usize) -> Result<(), String> {
    let list = vec!(r"Sun", r"199", r"299", r"399", r"499", r"599", r"699", r"799", r"899", r"999",
        r"A801 AA", r"A807 FA", r"A802 FA", r"301");
    let mut date = time::Date::try_from_ymd(1945,07,16).unwrap();
    println!("{} test system...", Colour::Blue.bold().paint("Building"));
    let mut test = System::fetch_from_horizons(&list, &date).unwrap();
    println!("{} simulation for {} days (this could take a while)", Colour::Blue.bold().paint("Running"), days);
    (0..days).into_iter().for_each(|_| {
        let hour = 1.0/24.0;
        let remainder = 1.0 - (hour * 23.0);
        (0..23).into_iter().for_each(|_| {
            test.kick_drift_kick_step(hour);
        });
        test.kick_drift_kick_step(remainder);
        date = date.next_day();
    });
    println!("{} control system...", Colour::Blue.bold().paint("Building"));
    let control = System::fetch_from_horizons(&list, &date).unwrap();
    println!("{} system body distances to star...", Colour::Yellow.bold().paint("Comparing"));
    (0..control.size()).into_iter().for_each(|x| {
        approximate_compare(
            &control.object_from_index(0).unwrap().read().unwrap(),
            &control.object_from_index(x).unwrap().read().unwrap(),
            &test.object_from_index(0).unwrap().read().unwrap(),
            &test.object_from_index(x).unwrap().read().unwrap())
    });
    Ok(())
}
#[test]
fn simulate_planets_for_day() -> Result<(), String> {
    simulate_planets_for(1)
}
#[test]
fn simulate_planets_for_week() -> Result<(), String> {
    simulate_planets_for(7)
}
#[test]
fn simulate_planets_for_month() -> Result<(), String> {
    simulate_planets_for(30)
}
#[test]
fn simulate_planets_for_quarter_year() -> Result<(), String> {
    simulate_planets_for(91)
}
#[test]
fn simulate_planets_for_half_year() -> Result<(), String> {
    simulate_planets_for(182)
}
#[test]
fn simulate_planets_for_year() -> Result<(), String> {
    simulate_planets_for(365)
}
#[test]
fn simulate_planets_for_quarter_decade() -> Result<(), String> {
    simulate_planets_for(913)
}
#[test]
fn simulate_planets_for_half_decade() -> Result<(), String> {
    simulate_planets_for(1826)
}
#[test]
fn simulate_planets_for_decade() -> Result<(), String> {
    simulate_planets_for(3652)
}
#[test]
fn simulate_planets_for_score() -> Result<(), String> {
    simulate_planets_for(9131)
}
#[test]
fn simulate_planets_for_half_century() -> Result<(), String> {
    simulate_planets_for(18262)
}
#[test]
fn simulate_planets_for_century() -> Result<(), String> {
    simulate_planets_for(36525)
}