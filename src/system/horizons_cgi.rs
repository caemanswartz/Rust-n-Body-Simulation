use time::Date;
use regex::Regex;
use super::body::Body;
pub fn fetch_target_body<T: AsRef<str>>(target: T, date: &Date) -> Result<Body, String> {
        let response = query_horizons_server_for(target, date).unwrap();
        generate_body_from(&response)
}
fn query_horizons_server_for<T: AsRef<str>>(target: T, date: &Date) -> Result<String, String> {
    let next_day: Date = date.next_day();
    let message = format!(
        "{}{}",
        "https://ssd.jpl.nasa.gov/horizons_batch.cgi?batch=1",
        format!("&COMMAND=%27{}%27\
            &CENTER=%27500@0%27\
            &MAKE_EPHEM=%27YES%27\
            &TABLE_TYPE=%27VECTORS%27\
            &START_TIME=%27{}%27\
            &STOP_TIME=%27{}%27\
            &STEP_SIZE=%271%27\
            &OUT_UNITS=%27AU-D%27\
            &REF_PLANE=%27ECLIPTIC%27\
            &REF_SYSTEM=%27J2000%27\
            &VECT_CORR=%27NONE%27\
            &VEC_LABELS=%27NO%27\
            &VEC_DELTA_T=%27NO%27\
            &CSV_FORMAT=%27YES%27\
            &OBJ_DATA=%27YES%27\
            &VEC_TABLE=%272%27",
            target.as_ref(),
            date.format("%Y-%m-%d"),
            next_day.format("%Y-%m-%d")
        )
    );
    let response = match reqwest::blocking::get(message) {
        Err(x) => return Err(format!("Failed to get response for {} from HORIZONS!\n{}", target.as_ref(), x)),
        Ok(x) => match x.text() {
            Err(x) => return Err(format!("Failed to parse response for {} from HORIZONS!\n{}", target.as_ref(), x)),
            Ok(x) => x
        }
    };
    Ok(response)
}
fn search_and_replace(regex: &str, sample: &str, target: &str) -> Result<String, String> {
    match Regex::new(regex) {
        Ok(x) => match x.captures(sample) {
            Some(y) => Ok(x.replace(&y[0], target).to_string()),
            None => Err(format!("Failed to find {} in {}", target, sample))
        },
        Err(x) => return Err(format!("Failed to parse regex {}!\n{}", regex, x))
    }
}
fn generate_body_from(response: &str) -> Result<Body,String> {
    let filtered: Vec<String> = response.split('*').filter(|&x| !x.is_empty()).map(|x| {x.to_string()}).collect();
    let target = match search_and_replace(
        r"((Revised:\s*\w+\s*\d+,\s*\d+\s*)|(JPL/\w+\s*))(?P<name>\d*\s?\w+(\s\w+)*)(\s/\s\(\w+\))?\s*\(?(?P<number>\w+\s?\w*)((\))|(\n\s?\n)|(\s/))",
        &filtered[0],
        "$name ($number)"
    ){
        Ok(x) => x,
        Err(x) => return Err(format!("Failed to find target name!\n{}", x))
    };
    let regex = r"(?P<x>(-?\+?)\d+.\d+E(\+?-?)\d+),\s+(?P<y>(-?\+?)\d+.\d+E(\+?-?)\d+),\s+(?P<z>(-?\+?)\d+.\d+E(\+?-?)\d+),\s*(?x)
        (?P<vx>(-?\+?)\d+.\d+E(\+?-?)\d+),\s+(?P<vy>(-?\+?)\d+.\d+E(\+?-?)\d+),\s+(?P<vz>(-?\+?)\d+.\d+E(\+?-?)\d+),\s*\n\d+";
    let vector = match search_and_replace(
        regex,
        &filtered[7],
        "$x,$y,$z,$vx,$vy,$vz")
    {
        Ok(x) => x,
        Err(_) => match search_and_replace(
            regex,
            &filtered[8],
            "$x,$y,$z,$vx,$vy,$vz")
        {
            Ok(x) => x,
            Err(x) => return Err(format!("Failed to find position and velocity for {}!\n{}", target, x))
        }
    };
    let iterator = vector.split(',');
    let floats: Vec<_> = iterator.map(|s| s.parse::<f64>().map_err(|err| err.to_string())).collect();
    let gravitational_mass: f64 = match match search_and_replace(
        r"GM,?\s*(\(planet\))?\s*(\(?km\^3/?\s*s\^-?2\)?)?\s*=\s*(?P<gm>\d+.\d+)",
        &filtered[0],
        "$gm")
    {
        Ok(x) => x,
        Err(x) => {
            println!("Failed to find GM for {}! Setting GM to 0.0...\n{}", target, x);
            "0.0".to_string()
        }
    }.parse::<f64>()
    {
        Ok(x) => x,
        Err(x) => {
            println!("Failed to parse GM for {}\n{}", target, x);
            0.0
        }
    };
    let radius: f64 = match search_and_replace(
        r"(?:Vol. [Mm]ean [Rr]adius,?\s*\(?(?:km)?\)?\s*=\s*)(?P<rad_dig>\d+)\.?(P<rad_dec>\d*)?",
        &filtered[0],
        "${rad_dig}.${rad_dec}0")
    {
        Ok(x) => match x.parse()
        {
            Ok(x) => x,
            Err(x) => {
                println!("Failed to parse radius for {}! Setting radius to 0.0...{}",target, x);
                0.0
            }
        },
        Err(_) => {
            println!("Failed to find radius for {}! Looking for density...", target);
            match search_and_replace(
                r"(?:(?:Mean)? [Dd]ensity),?\s*((\(R=\d+\s*\D+\))|(\(?g/?\s*cm\^-?3\)?))\s*=\s*(?P<density>\d+\.\d+)",
                &filtered[0],
                "$density") {
                Ok(x) => match x.parse::<u64>() {
                    Ok(x) => (3.0 * ((gravitational_mass * 6.67430E-17) / x as f64) / (4.0 * std::f64::consts::PI) ).powf(1.0/3.0),
                    Err(x) => {
                        println!("Failed to parse density for {}! Setting radius to 0.0...\n{}",target, x);
                        0.0
                    }
                },
                Err(_) => {
                    println!("Failed to find density for {}! Setting radius to 0.0", target);
                    0.0
                }
            }
        }
    };
    Ok(Body::new(
            target.into(),
            gravitational_mass,
            radius,
            [
                floats[0].clone()?,
                floats[1].clone()?,
                floats[2].clone()?
            ],
            [
                floats[3].clone()?,
                floats[4].clone()?,
                floats[5].clone()?
            ],
    ))
}
pub fn fetch_target_bodies<T: AsRef<str>>(target: &[T], date: &Date) -> Vec<Body> {
    target.iter().map(|a| {
        println!("Fetching {}...", a.as_ref());
        match fetch_target_body(a, date) {
            Ok(x) => Ok(x),
            Err(x) => {
                println!("Failed to fetch {}! Skipping...\n{}", a.as_ref(), x);
                Err(x)
            }
        }
    }).filter(|a| {
        match a {
            Ok(_) => true,
            Err(_) => false
        }
    }).map(|a| {a.unwrap()}).collect()
}
#[cfg(test)]
mod test {
    use super::*;
    use float_eq::assert_float_eq;
    fn compare(control: &Body, test: &Body) {
        assert_eq!(control.name(), test.name());
        assert_eq!(control.mass(), test.mass());
        assert_float_eq!(control.position(), test.position(), ulps <= [1,1,1]);
        assert_float_eq!(control.velocity(), test.velocity(), ulps <= [1,1,1]);
    }
    fn compare_sun_to(test: &Body) {
        let control = Body::new(
            "Sun (10)".to_string(),
            132712440041.93938,
            695700.0,
                [
                    4.494340582683912E-03,
                    9.104614297180857E-04,
                    -6.099490045495054E-05
                ],
                [
                    -4.728900304182371E-07,
                    5.597222756099664E-06,
                    -1.295971036475890E-08
                ]
        );
        compare(&control, test);
    }
    fn compare_mercury_to(test: &Body) {
        let control = Body::new(
            "Mercury (199)".to_string(),
            22031.86855,
            2440.0,
                [
                    6.070711234471207E-02,
                    3.026468028702081E-01,
                    1.941189349867229E-02
                ],
                [
                    -3.329788198291871E-02,
                    6.209252911230533E-03,
                    3.565339249484719E-03
                ]
        );
        compare(&control, test);
    }
    fn compare_earth_to(test: &Body) {
        let control = Body::new(
            "Earth (399)".to_string(),
            398600.435436,
            6371.01,
                [
                    4.133060075292528E-01,
                    -9.296817278172866E-01,
                    -1.236944559827514E-04
                ],
                [
                    1.547590112466981E-02,
                    6.866255831713478E-03,
                    9.706289312687370E-07
                ]
        );
        compare(&control, test);
    }
    fn compare_ceres_to(test: &Body) {
        let control = Body::new(
            "1 Ceres (A801 AA)".to_string(),
            62.6284,
            722.33676353301,
                [
                    1.592773884234155E+00,
                    -2.463766259162856E+00,
                    -3.653478442536497E-01
                ],
                [
                    8.151272151318315E-03,
                    5.008550137955732E-03,
                    -1.362500743964101E-03
                ]
        );
        compare(&control, test);
    }
    fn compare_jupiter_to(test: &Body) {
        let control = Body::new(
            "Jupiter (599)".to_string(),
            126686531.9,
            69911.0,
                [
                    -5.358460001405257E+00,
                    -9.789605937582654E-01,
                    1.241187624618316E-01
                ],
                [
                    1.268369810401409E-03,
                    -7.071526885645118E-03,
                    6.786513273345366E-07
                ]
        );
        compare(&control, test);
    }
    #[test]
    fn generate_sun() -> Result<(), String> {
        let response = "*************************************************************\n Revised: July 31, 2013                  Sun                                 10\n\n PHYSICAL PROPERTIES (updated 2018-Aug-15):\n  GM, km^3/s^2          = 132712440041.93938  Mass, 10^24 kg        = ~1988500\n  Vol. mean radius, km  = 695700              Volume, 10^12 km^3    = 1412000\n  Solar radius (IAU)    = 696000 km           Mean density, g/cm^3  = 1.408\n  Radius (photosphere)  = 696500 km           Angular diam at 1 AU  = 1919.3\"\n  Photosphere temp., K  = 6600 (bottom)       Photosphere temp., K  = 4400(top)\n  Photospheric depth    = ~500 km             Chromospheric depth   = ~2500 km\n  Flatness, f           = 0.00005             Adopted sid. rot. per.= 25.38 d\n  Surface gravity       =  274.0 m/s^2        Escape speed, km/s    =  617.7\n  Pole (RA,DEC), deg.   = (286.13, 63.87)     Obliquity to ecliptic = 7.25 deg.\n  Solar constant (1 AU) = 1367.6 W/m^2        Luminosity, 10^24 J/s = 382.8\n  Mass-energy conv rate = 4.260 x 10^9 kg/s   Effective temp, K     = 5772\n  Sunspot cycle         = 11.4 yr             Cycle 24 sunspot min. = 2008 A.D.\n\n  Motion relative to nearby stars = apex : R.A.= 271 deg.; DEC.= +30 deg.\n                                    speed: 19.4 km/s (0.0112 au/day)\n  Motion relative to 2.73K BB/CBR = apex : l= 264.7 +- 0.8; b= 48.2 +- 0.5 deg.\n                                    speed: 369 +-11 km/s\n*******************************************************************************\n \n \n*******************************************************************************\nEphemeris / WWW_USER Sun Apr 18 16:19:37 2021 Pasadena, USA      / Horizons\n*******************************************************************************\nTarget body name: Sun (10)                        {source: DE441}\nCenter body name: Solar System Barycenter (0)     {source: DE441}\nCenter-site name: BODY CENTER\n*******************************************************************************\nStart time      : A.D. 1969-Jul-16 00:00:00.0000 TDB\nStop  time      : A.D. 1969-Jul-17 00:00:00.0000 TDB\nStep-size       : 1 steps\n*******************************************************************************\nCenter geodetic : 0.00000000,0.00000000,0.0000000 {E-lon(deg),Lat(deg),Alt(km)}\nCenter cylindric: 0.00000000,0.00000000,0.0000000 {E-lon(deg),Dxy(km),Dz(km)}\nCenter radii    : (undefined)                                                  \nOutput units    : AU-D\nOutput type     : GEOMETRIC cartesian states\nOutput format   : 2 (position and velocity)\nReference frame : Ecliptic of J2000.0\n*******************************************************************************\n            JDTDB,            Calendar Date (TDB),                      X,                      Y,                      Z,                     VX,                     VY,                     VZ,\n**************************************************************************************************************************************************************************************************\n$$SOE\n2440418.500000000, A.D. 1969-Jul-16 00:00:00.0000,  4.494340582683912E-03,  9.104614297180857E-04, -6.099490045495054E-05, -4.728900304182371E-07,  5.597222756099664E-06, -1.295971036475890E-08,\n2440419.500000000, A.D. 1969-Jul-17 00:00:00.0000,  4.493864315235150E-03,  9.160578371287067E-04, -6.100779591719411E-05, -4.796512178557295E-07,  5.595598239330858E-06, -1.283021856753520E-08,\n$$EOE\n**************************************************************************************************************************************************************************************************\nCoordinate system description:\n\n  Ecliptic at the standard reference epoch\n\n    Reference epoch: J2000.0\n    X-Y plane: adopted Earth orbital plane at the reference epoch\n               Note: obliquity of 84381.448 arcseconds (IAU76) wrt ICRF equator\n    X-axis   : ICRF\n    Z-axis   : perpendicular to the X-Y plane in the directional (+ or -) sense\n               of Earth's north pole at the reference epoch.\n\n  Symbol meaning [1 au= 149597870.700 km, 1 day= 86400.0 s]:\n\n    JDTDB    Julian Day Number, Barycentric Dynamical Time\n      X      X-component of position vector (au)\n      Y      Y-component of position vector (au)\n      Z      Z-component of position vector (au)\n      VX     X-component of velocity vector (au/day)                           \n      VY     Y-component of velocity vector (au/day)                           \n      VZ     Z-component of velocity vector (au/day)                           \n\nGeometric states/elements have no aberrations applied.\n\n\n Computations by ...\n     Solar System Dynamics Group, Horizons On-Line Ephemeris System\n     4800 Oak Grove Drive, Jet Propulsion Laboratory\n     Pasadena, CA  91109   USA\n     Information  : https://ssd.jpl.nasa.gov/\n     Documentation: https://ssd.jpl.nasa.gov/?horizons_doc\n     Connect      : https://ssd.jpl.nasa.gov/?horizons (browser)\n                    telnet ssd.jpl.nasa.gov 6775       (command-line)\n                    e-mail command interface available\n                    Script and CGI interfaces available\n     Author       : Jon.D.Giorgini@jpl.nasa.gov\n*******************************************************************************";
        let test = generate_body_from(response).unwrap();
        compare_sun_to(&test);
        Ok(())
    }
    #[test]
    fn generate_mercury() -> Result<(), String> {
        let response = "*******************************************************************************\n Revised: April 12, 2021             Mercury                            199 / 1\n\n PHYSICAL DATA (updated 2021-Apr-12):\n  Vol. Mean Radius (km) =  2440+-1        Density (g cm^-3)     = 5.427\n  Mass x10^23 (kg)      =     3.302       Volume (x10^10 km^3)  = 6.085\n  Sidereal rot. period  =    58.6463 d    Sid. rot. rate (rad/s)= 0.00000124001\n  Mean solar day        =   175.9421 d    Core radius (km)      = ~1600\n  Geometric Albedo      =     0.106       Surface emissivity    = 0.77+-0.06\n  GM (km^3/s^2)         = 22031.86855     Equatorial radius, Re = 2440 km\n  GM 1-sigma (km^3/s^2) =                 Mass ratio (Sun/plnt) = 6023682\n  Mom. of Inertia       =     0.33        Equ. gravity  m/s^2   = 3.701\n  Atmos. pressure (bar) = < 5x10^-15      Max. angular diam.    = 11.0\"\n  Mean Temperature (K)  = 440             Visual mag. V(1,0)    = -0.42\n  Obliquity to orbit[1] =  2.11' +/- 0.1' Hill's sphere rad. Rp = 94.4\n  Sidereal orb. per.    =  0.2408467 y    Mean Orbit vel.  km/s = 47.362\n  Sidereal orb. per.    = 87.969257  d    Escape vel. km/s      =  4.435\n                                 Perihelion  Aphelion    Mean\n  Solar Constant (W/m^2)         14462       6278        9126\n  Maximum Planetary IR (W/m^2)   12700       5500        8000\n  Minimum Planetary IR (W/m^2)   6           6           6\n*******************************************************************************\n\n\n*******************************************************************************\nEphemeris / WWW_USER Thu Apr 15 21:07:43 2021 Pasadena, USA      / Horizons\n*******************************************************************************\nTarget body name: Mercury (199)                   {source: DE441}\nCenter body name: Solar System Barycenter (0)     {source: DE441}\nCenter-site name: BODY CENTER\n*******************************************************************************\nStart time      : A.D. 1969-Jul-16 00:00:00.0000 TDB\nStop  time      : A.D. 1969-Jul-17 00:00:00.0000 TDB\nStep-size       : 1 steps\n*******************************************************************************\nCenter geodetic : 0.00000000,0.00000000,0.0000000 {E-lon(deg),Lat(deg),Alt(km)}\nCenter cylindric: 0.00000000,0.00000000,0.0000000 {E-lon(deg),Dxy(km),Dz(km)}\nCenter radii    : (undefined)\nOutput units    : AU-D\nOutput type     : GEOMETRIC cartesian states\nOutput format   : 2 (position and velocity)\nReference frame : Ecliptic of J2000.0\n*******************************************************************************\n            JDTDB,            Calendar Date (TDB),                      X,                      Y,                      Z,       \n              VX,                     VY,                     VZ,\n**************************************************************************************************************************************************************************************************\n$$SOE\n2440418.500000000, A.D. 1969-Jul-16 00:00:00.0000,  6.070711234471207E-02,  3.026468028702081E-01,  1.941189349867229E-02, -3.329788198291871E-02,  6.209252911230533E-03,  3.565339249484719E-03,\n2440419.500000000, A.D. 1969-Jul-17 00:00:00.0000,  2.718021949636515E-02,  3.073140462154216E-01,  2.287236510774198E-02, -3.369882929656001E-02,  3.120089688655550E-03,  3.349940883840557E-03,\n$$EOE\n**************************************************************************************************************************************************************************************************\nCoordinate system description:\n\n  Ecliptic at the standard reference epoch\n\n    Reference epoch: J2000.0\n    X-Y plane: adopted Earth orbital plane at the reference epoch\n               Note: obliquity of 84381.448 arcseconds (IAU76) wrt ICRF equator\n    X-axis   : ICRF\n    Z-axis   : perpendicular to the X-Y plane in the directional (+ or -) sense\n               of Earth's north pole at the reference epoch.\n\n  Symbol meaning [1 au= 149597870.700 km, 1 day= 86400.0 s]:\n\n    JDTDB    Julian Day Number, Barycentric Dynamical Time\n      X      X-component of position vector (au)\n      Y      Y-component of position vector (au)\n      Z      Z-component of position vector (au)\n      VX     X-component of velocity vector (au/day)\n      VY     Y-component of velocity vector (au/day)\n      VZ     Z-component of velocity vector (au/day)\n\nGeometric states/elements have no aberrations applied.\n\n\n Computations by ...\n     Solar System Dynamics Group, Horizons On-Line Ephemeris System\n     4800 Oak Grove Drive, Jet Propulsion Laboratory\n     Pasadena, CA  91109   USA\n     Information  : https://ssd.jpl.nasa.gov/\n     Documentation: https://ssd.jpl.nasa.gov/?horizons_doc\n     Connect      : https://ssd.jpl.nasa.gov/?horizons (browser)\n                    telnet ssd.jpl.nasa.gov 6775       (command-line)\n                    e-mail command interface available\n                    Script and CGI interfaces available\n     Author       : Jon.D.Giorgini@jpl.nasa.gov\n*******************************************************************************\n\n!$$SOF\nCOMMAND = '199'\nCENTER = '500@0'\nMAKE_EPHEM = 'YES'\nTABLE_TYPE = 'VECTORS'\nSTART_TIME = '1969-07-16'\nSTOP_TIME = '1969-07-17'\nSTEP_SIZE = '1'\nOUT_UNITS = 'AU-D'\nREF_PLANE = 'ECLIPTIC'\nREF_SYSTEM = 'J2000'\nVECT_CORR = 'NONE'\nVEC_LABELS = 'NO'\nVEC_DELTA_T = 'NO'\nCSV_FORMAT = 'YES'\nOBJ_DATA = 'YES'\nVEC_TABLE = '2'";
        let test = generate_body_from(response).unwrap();
        compare_mercury_to(&test);
        Ok(())
    }
    #[test]
    fn generate_earth() -> Result <(), String> {
        let response = "*******************************************************************************\n Revised: April 12, 2021                 Earth                              399\n \n GEOPHYSICAL PROPERTIES (revised Aug 15, 2018):\n  Vol. Mean Radius (km)    = 6371.01+-0.02   Mass x10^24 (kg)= 5.97219+-0.0006\n  Equ. radius, km          = 6378.137        Mass layers:\n  Polar axis, km           = 6356.752          Atmos         = 5.1   x 10^18 kg\n  Flattening               = 1/298.257223563   oceans        = 1.4   x 10^21 kg\n  Density, g/cm^3          = 5.51              crust         = 2.6   x 10^22 kg\n  J2 (IERS 2010)           = 0.00108262545     mantle        = 4.043 x 10^24 kg\n  g_p, m/s^2  (polar)      = 9.8321863685      outer core    = 1.835 x 10^24 kg\n  g_e, m/s^2  (equatorial) = 9.7803267715      inner core    = 9.675 x 10^22 kg\n  g_o, m/s^2               = 9.82022         Fluid core rad  = 3480 km\n  GM, km^3/s^2             = 398600.435436   Inner core rad  = 1215 km\n  GM 1-sigma, km^3/s^2     =      0.0014     Escape velocity = 11.186 km/s\n  Rot. Rate (rad/s)        = 0.00007292115   Surface area:\n  Mean sidereal day, hr    = 23.9344695944     land          = 1.48 x 10^8 km\n  Mean solar day 2000.0, s = 86400.002         sea           = 3.62 x 10^8 km\n  Mean solar day 1820.0, s = 86400.0         Love no., k2    = 0.299\n  Moment of inertia        = 0.3308          Atm. pressure   = 1.0 bar\n  Mean temperature, K      = 270             Volume, km^3    = 1.08321 x 10^12\n  Mean effect. IR temp, K  = 255             Magnetic moment = 0.61 gauss Rp^3\n  Geometric albedo         = 0.367           Vis. mag. V(1,0)= -3.86\n  Solar Constant (W/m^2)   = 1367.6 (mean), 1414 (perihelion), 1322 (aphelion)\n HELIOCENTRIC ORBIT CHARACTERISTICS:\n  Obliquity to orbit, deg  = 23.4392911  Sidereal orb period  = 1.0000174 y\n  Orbital speed, km/s      = 29.79       Sidereal orb period  = 365.25636 d\n  Mean daily motion, deg/d = 0.9856474   Hill's sphere radius = 234.9       \n*******************************************************************************\n \n \n*******************************************************************************\nEphemeris / WWW_USER Sun Apr 18 18:23:05 2021 Pasadena, USA      / Horizons\n*******************************************************************************\nTarget body name: Earth (399)                     {source: DE441}\nCenter body name: Solar System Barycenter (0)     {source: DE441}\nCenter-site name: BODY CENTER\n*******************************************************************************\nStart time      : A.D. 1969-Jul-16 00:00:00.0000 TDB\nStop  time      : A.D. 1969-Jul-17 00:00:00.0000 TDB\nStep-size       : 1 steps\n*******************************************************************************\nCenter geodetic : 0.00000000,0.00000000,0.0000000 {E-lon(deg),Lat(deg),Alt(km)}\nCenter cylindric: 0.00000000,0.00000000,0.0000000 {E-lon(deg),Dxy(km),Dz(km)}\nCenter radii    : (undefined)                                                  \nOutput units    : AU-D\nOutput type     : GEOMETRIC cartesian states\nOutput format   : 2 (position and velocity)\nReference frame : Ecliptic of J2000.0\n*******************************************************************************\n            JDTDB,            Calendar Date (TDB),                      X,                      Y,                      Z,                     VX,                     VY,                     VZ,\n**************************************************************************************************************************************************************************************************\n$$SOE\n2440418.500000000, A.D. 1969-Jul-16 00:00:00.0000,  4.133060075292528E-01, -9.296817278172866E-01, -1.236944559827514E-04,  1.547590112466981E-02,  6.866255831713478E-03,  9.706289312687370E-07,\n2440419.500000000, A.D. 1969-Jul-17 00:00:00.0000,  4.287230660560534E-01, -9.226841268375744E-01, -1.226723981466474E-04,  1.535744988472102E-02,  7.128590328755608E-03,  1.069934491893486E-06,\n$$EOE\n**************************************************************************************************************************************************************************************************\nCoordinate system description:\n\n  Ecliptic at the standard reference epoch\n\n    Reference epoch: J2000.0\n    X-Y plane: adopted Earth orbital plane at the reference epoch\n               Note: obliquity of 84381.448 arcseconds (IAU76) wrt ICRF equator\n    X-axis   : ICRF\n    Z-axis   : perpendicular to the X-Y plane in the directional (+ or -) sense\n               of Earth's north pole at the reference epoch.\n\n  Symbol meaning [1 au= 149597870.700 km, 1 day= 86400.0 s]:\n\n    JDTDB    Julian Day Number, Barycentric Dynamical Time\n      X      X-component of position vector (au)\n      Y      Y-component of position vector (au)\n      Z      Z-component of position vector (au)\n      VX     X-component of velocity vector (au/day)                           \n      VY     Y-component of velocity vector (au/day)                           \n      VZ     Z-component of velocity vector (au/day)                           \n\nGeometric states/elements have no aberrations applied.\n\n\n Computations by ...\n     Solar System Dynamics Group, Horizons On-Line Ephemeris System\n     4800 Oak Grove Drive, Jet Propulsion Laboratory\n     Pasadena, CA  91109   USA\n     Information  : https://ssd.jpl.nasa.gov/\n     Documentation: https://ssd.jpl.nasa.gov/?horizons_doc\n     Connect      : https://ssd.jpl.nasa.gov/?horizons (browser)\n                    telnet ssd.jpl.nasa.gov 6775       (command-line)\n                    e-mail command interface available\n                    Script and CGI interfaces available\n     Author       : Jon.D.Giorgini@jpl.nasa.gov\n*******************************************************************************";
        let test = generate_body_from(response).unwrap();
        compare_earth_to(&test);
        Ok(())
    }
    #[test]
    fn generate_asteroid() -> Result<(), String> {
        let response = "*******************************************************************************\nJPL/HORIZONS                  1 Ceres (A801 AA)            2021-Apr-18 18:12:43\nRec #:       1 (+COV) Soln.date: 2021-Apr-13_11:04:44   # obs: 1075 (1995-2021)\n \nIAU76/J2000 helio. ecliptic osc. elements (au, days, deg., period=Julian yrs):\n \n  EPOCH=  2458849.5 ! 2020-Jan-01.00 (TDB)         Residual RMS= .24563\n   EC= .07687465013145245  QR= 2.556401146697176   TP= 2458240.1791309435\n   OM= 80.3011901917491    W=  73.80896808746482   IN= 10.59127767086216\n   A= 2.769289292143484    MA= 130.3159688200986   ADIST= 2.982177437589792\n   PER= 4.60851            N= .213870839           ANGMOM= .028541613\n   DAN= 2.69515            DDN= 2.81323            L= 153.8445988\n   B= 10.1666388           MOID= 1.59231997        TP= 2018-May-01.6791309435\n \nAsteroid physical parameters (km, seconds, rotational period in hours):\n   GM= 62.6284             RAD= 469.7              ROTPER= 9.07417\n   H= 3.53                 G= .120                 B-V= .713\n                           ALBEDO= .090            STYP= C\n \nASTEROID comments: \n1: soln ref.= JPL#48, OCC=0           radar(60 delay, 0 Dop.)\n2: source=ORB\n*******************************************************************************\n \n \n*******************************************************************************\nEphemeris / WWW_USER Sun Apr 18 18:12:43 2021 Pasadena, USA      / Horizons\n*******************************************************************************\nTarget body name: 1 Ceres (A801 AA)               {source: JPL#48}\nCenter body name: Solar System Barycenter (0)     {source: DE431}\nCenter-site name: BODY CENTER\n*******************************************************************************\nStart time      : A.D. 1969-Jul-16 00:00:00.0000 TDB\nStop  time      : A.D. 1969-Jul-17 00:00:00.0000 TDB\nStep-size       : 1 steps\n*******************************************************************************\nCenter geodetic : 0.00000000,0.00000000,0.0000000 {E-lon(deg),Lat(deg),Alt(km)}\nCenter cylindric: 0.00000000,0.00000000,0.0000000 {E-lon(deg),Dxy(km),Dz(km)}\nCenter radii    : (undefined)                                                  \nSmall perturbers: Yes                             {source: SB431-N16}\nOutput units    : AU-D\nOutput type     : GEOMETRIC cartesian states\nOutput format   : 2 (position and velocity)\nReference frame : Ecliptic of J2000.0\n*******************************************************************************\nInitial IAU76/J2000 heliocentric ecliptic osculating elements (au, days, deg.):\n  EPOCH=  2458849.5 ! 2020-Jan-01.00 (TDB)         Residual RMS= .24563        \n   EC= .07687465013145245  QR= 2.556401146697176   TP= 2458240.1791309435      \n   OM= 80.3011901917491    W=  73.80896808746482   IN= 10.59127767086216       \n  Equivalent ICRF heliocentric cartesian coordinates (au, au/d):\n   X= 1.007608869627324E+00  Y=-2.390064275218395E+00  Z=-1.332124522752835E+00\n  VX= 9.201724467231788E-03 VY= 3.370381135450014E-03 VZ=-2.850337057427248E-04\nAsteroid physical parameters (km, seconds, rotational period in hours):        \n   GM= 62.6284             RAD= 469.7              ROTPER= 9.07417             \n   H= 3.53                 G= .120                 B-V= .713                   \n                           ALBEDO= .090            STYP= C                     \n*******************************************************************************\n            JDTDB,            Calendar Date (TDB),                      X,                      Y,                      Z,                     VX,                     VY,                     VZ,\n**************************************************************************************************************************************************************************************************\n$$SOE\n2440418.500000000, A.D. 1969-Jul-16 00:00:00.0000,  1.592773884234155E+00, -2.463766259162856E+00, -3.653478442536497E-01,  8.151272151318315E-03,  5.008550137955732E-03, -1.362500743964101E-03,\n2440419.500000000, A.D. 1969-Jul-17 00:00:00.0000,  1.600916030101704E+00, -2.458743583559882E+00, -3.667082473956584E-01,  8.133005182370315E-03,  5.036789680357272E-03, -1.358303206642045E-03,\n$$EOE\n**************************************************************************************************************************************************************************************************\nCoordinate system description:\n\n  Ecliptic at the standard reference epoch\n\n    Reference epoch: J2000.0\n    X-Y plane: adopted Earth orbital plane at the reference epoch\n               Note: obliquity of 84381.448 arcseconds (IAU76) wrt ICRF equator\n    X-axis   : ICRF\n    Z-axis   : perpendicular to the X-Y plane in the directional (+ or -) sense\n               of Earth's north pole at the reference epoch.\n\n  Symbol meaning [1 au= 149597870.700 km, 1 day= 86400.0 s]:\n\n    JDTDB    Julian Day Number, Barycentric Dynamical Time\n      X      X-component of position vector (au)\n      Y      Y-component of position vector (au)\n      Z      Z-component of position vector (au)\n      VX     X-component of velocity vector (au/day)                           \n      VY     Y-component of velocity vector (au/day)                           \n      VZ     Z-component of velocity vector (au/day)                           \n\nGeometric states/elements have no aberrations applied.\n\n\n Computations by ...\n     Solar System Dynamics Group, Horizons On-Line Ephemeris System\n     4800 Oak Grove Drive, Jet Propulsion Laboratory\n     Pasadena, CA  91109   USA\n     Information  : https://ssd.jpl.nasa.gov/\n     Documentation: https://ssd.jpl.nasa.gov/?horizons_doc\n     Connect      : https://ssd.jpl.nasa.gov/?horizons (browser)\n                    telnet ssd.jpl.nasa.gov 6775       (command-line)\n                    e-mail command interface available\n                    Script and CGI interfaces available\n     Author       : Jon.D.Giorgini@jpl.nasa.gov\n*******************************************************************************"; // cSpell:enable
        let test = generate_body_from(response).unwrap();
        compare_ceres_to(&test);
        Ok(())
    }
    #[test]
    fn fetch_targets() -> Result<(), String> {
        let date = Date::try_from_ymd(1969,7,16).unwrap();
        let test = fetch_target_body(r"sun", &date)?;
        compare_sun_to(&test);
        let test = fetch_target_body(r"199", &date)?;
        compare_mercury_to(&test);
        let test = fetch_target_body(r"399", &date)?;
        compare_earth_to(&test);
        let test = fetch_target_body(r"A801 AA", &date)?;
        compare_ceres_to(&test);
        let test = fetch_target_body(r"599", &date)?;
        compare_jupiter_to(&test);
        Ok(())
    }
    #[test]
    fn fetch_star_and_planets() -> Result<(), String> {
        let targets = fetch_target_bodies(
            &vec!(
                "10",
                "199",
                "299",
                "399",
                "499",
                "599",
                "699",
                "799",
                "899",
                "134340"
            ),
            &Date::try_from_ymd(1969, 7, 16).unwrap()
        );
        let control = [
            "Sun (10)",
            "Mercury (199)",
            "Venus (299)",
            "Earth (399)",
            "Mars (499)",
            "Jupiter (599)",
            "Saturn (699)",
            "Uranus (799)",
            "Neptune (899)",
            "134340 Pluto (999)"
        ];
        let mut names = Vec::new();
        for target in targets {
            println!("fetching {:?}", target);
            names.push(target.name().to_string());
        }
        assert_eq!(names, control);
        Ok(())
    }
}