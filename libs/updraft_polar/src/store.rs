use updraft_units::{Area, Mass};

use crate::{GlidePolar, PolarCoefficients};

/// A glider from the built-in polar library.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolarStoreEntry {
    /// Glider model name. Names are not necessarily unique: a model can
    /// appear once per competition class (span configuration).
    pub name: &'static str,
    /// Quadratic sink coefficients in reduced form: sink in m/s
    /// (positive down) at an airspeed given in multiples of 100 km/h,
    /// valid at the reference mass with a clean wing.
    coefficients: (f64, f64, f64),
    /// The mass the coefficients were measured at (glider plus standard
    /// pilot, no ballast).
    pub reference_mass: Mass,
    /// Maximum take-off mass, where known.
    pub max_takeoff_mass: Option<Mass>,
    /// Empty mass, where known.
    pub empty_mass: Option<Mass>,
    /// Wing area.
    pub wing_area: Area,
}

impl PolarStoreEntry {
    /// This entry's coefficients converted to the SI form used by
    /// [`PolarCoefficients`].
    pub fn coefficients(&self) -> PolarCoefficients {
        /// The conversion factor from LX-like coefficients to SI.
        const CONVERSION_FACTOR: f64 = 100. / 3.6;

        let (a, b, c) = self.coefficients;
        let a = a / (CONVERSION_FACTOR * CONVERSION_FACTOR);
        let b = b / CONVERSION_FACTOR;
        PolarCoefficients::new(a, b, c).expect("built-in polar coefficients are valid")
    }

    /// The glide polar for this entry, at reference mass with a clean
    /// wing.
    pub fn glide_polar(&self) -> GlidePolar {
        GlidePolar::new(self.coefficients(), self.reference_mass)
            .expect("built-in polar reference mass is valid")
    }
}

const fn entry(
    name: &'static str,
    coefficients: (f64, f64, f64),
    reference_mass_kg: f64,
    max_takeoff_mass_kg: Option<f64>,
    empty_mass_kg: Option<f64>,
    wing_area_m2: f64,
) -> PolarStoreEntry {
    const fn mass(kilograms: Option<f64>) -> Option<Mass> {
        match kilograms {
            Some(kilograms) => Some(Mass::from_kilograms(kilograms)),
            None => None,
        }
    }

    PolarStoreEntry {
        name,
        coefficients,
        reference_mass: Mass::from_kilograms(reference_mass_kg),
        max_takeoff_mass: mass(max_takeoff_mass_kg),
        empty_mass: mass(empty_mass_kg),
        wing_area: Area::from_square_meters(wing_area_m2),
    }
}

/// The built-in glide polar library, sorted by name.
///
/// A few entries quote a reference mass above their maximum take-off
/// mass, kept as provided by the source data.
#[allow(clippy::approx_constant)]
#[rustfmt::skip]
pub const POLAR_STORE: &[PolarStoreEntry] = &[
    entry("AC 4C Russia", (2.07, -2.98, 1.73), 254., None, Some(140.), 7.7),
    entry("AS 33Es 15m", (0.95, -1.7, 1.35), 400., Some(550.), Some(290.), 8.8),
    entry("AS 33Es 18m", (0.98, -1.61, 1.14), 400., Some(600.), Some(300.), 10.),
    entry("AS 34Me 15m", (1.515, -2.99, 2.18), 450., Some(525.), Some(290.), 10.5),
    entry("AS 34Me 18m", (1.46, -2.66, 1.79), 480., Some(575.), Some(300.), 11.88),
    entry("ASG 29 15m", (1.48, -2.4, 1.54), 322., Some(550.), Some(270.), 9.2),
    entry("ASG 29 18m", (1.33, -2.03, 1.24), 388., Some(600.), Some(280.), 10.5),
    entry("ASG 29E 15m", (1.35, -2.4, 1.69), 350., Some(550.), Some(295.), 9.2),
    entry("ASG 29E 18m", (1.31, -2.07, 1.29), 409., Some(600.), Some(325.), 10.5),
    entry("ASG 32Mi", (1.25, -2.02, 1.38), 650., Some(850.), Some(545.), 15.7),
    entry("ASH 25", (1.05, -1.69, 1.11), 685., Some(750.), Some(470.), 16.31),
    entry("ASH 25E", (1.05, -1.69, 1.11), 693., Some(750.), Some(478.), 16.5),
    entry("ASH 26", (1.05, -1.42, 0.93), 420., Some(525.), Some(270.), 11.68),
    entry("ASH 26E", (1.46, -2.53, 1.64), 397., Some(525.), Some(360.), 11.68),
    entry("ASH 30Mi", (0.81, -1.29, 0.96), 831., Some(850.), Some(630.), 17.17),
    entry("ASH 31/18m", (0.98, -1.45, 1.), 513., Some(630.), Some(420.), 11.9),
    entry("ASH 31/21m", (1.37, -2.41, 1.53), 512., Some(700.), Some(430.), 13.2),
    entry("ASK 13", (3.02, -4.36, 2.38), 525., Some(480.), Some(290.), 17.5),
    entry("ASK 14", (2.86, -3.8, 2.03), 330., Some(360.), Some(245.), 12.68),
    entry("ASK 16", (3.08, -4.83, 2.89), 700., Some(750.), Some(530.), 19.),
    entry("ASK 21", (2.5, -4.08, 2.4), 538., Some(600.), Some(360.), 17.95),
    entry("ASK 23", (2.15, -3.04, 1.74), 387., Some(360.), Some(240.), 12.9),
    entry("ASW 15", (2.04, -2.9, 1.66), 330., Some(408.), Some(230.), 11.),
    entry("ASW 17", (1.27, -1.94, 1.3), 444., Some(610.), Some(415.), 14.8),
    entry("ASW 19", (1.66, -2.36, 1.45), 330., Some(408.), Some(250.), 11.),
    entry("ASW 20", (1.18, -1.53, 1.), 315., Some(525.), Some(260.), 10.5),
    entry("ASW 20C", (1.02, -1.26, 0.91), 315., Some(454.), Some(260.), 10.5),
    entry("ASW 22-22m", (1.48, -2.39, 1.44), 480., Some(750.), Some(400.), 14.9),
    entry("ASW 22-24m", (1.29, -1.81, 1.05), 500., Some(650.), Some(410.), 15.5),
    entry("ASW 22BE", (1.09, -1.58, 0.96), 555., Some(810.), Some(545.), 16.31),
    entry("ASW 22BL", (1.31, -1.86, 1.05), 500., Some(750.), Some(459.), 16.67),
    entry("ASW 22BLE", (1.09, -1.58, 0.96), 555., Some(810.), Some(550.), 16.31),
    entry("ASW 24", (1.58, -2.21, 1.27), 310., Some(500.), Some(230.), 10.),
    entry("ASW 27B", (1.48, -2.4, 1.54), 315., Some(500.), Some(245.), 9.),
    entry("ASW 28-15", (1.53, -2.3, 1.43), 325., Some(525.), Some(258.), 10.5),
    entry("ASW 28-18", (1.72, -2.73, 1.58), 345., Some(575.), Some(270.), 11.88),
    entry("Antares 18S", (2.06, -4.09, 2.57), 351., Some(600.), Some(280.), 10.97),
    entry("Antares 18T", (1.24, -1.96, 1.26), 395., Some(600.), Some(325.), 10.97),
    entry("Antares 20E", (1.12, -2.01, 1.41), 529., Some(660.), Some(460.), 12.6),
    entry("Antares 23E", (0.74, -1.26, 1.), 560., Some(850.), Some(510.), 14.75),
    entry("Antares 23T", (0.9, -1.29, 0.86), 500., Some(850.), Some(510.), 14.75),
    entry("Apis 13m", (1.71, -2.43, 1.46), 207., Some(245.), Some(135.), 10.36),
    entry("Apis bee", (1.73, -2.8, 1.73), 318., Some(350.), Some(215.), 12.24),
    entry("Arcus", (0.91, -1.43, 1.12), 700., Some(800.), Some(430.), 15.6),
    entry("Arcus M", (0.91, -1.43, 1.12), 700., Some(800.), Some(500.), 15.6),
    entry("Arcus T", (0.91, -1.43, 1.12), 700., Some(800.), Some(470.), 15.6),
    entry("Astir CS", (2.04, -3.09, 1.86), 372., Some(450.), Some(255.), 12.4),
    entry("Blanik L13", (2.66, -3.57, 1.99), 472., Some(500.), Some(292.), 19.15),
    entry("Blanik L23", (2.45, -3.14, 1.75), 491., Some(530.), Some(310.), 19.2),
    entry("Blanik L33", (2.45, -3.14, 1.75), 281., Some(340.), Some(215.), 11.),
    entry("Capstan T49B", (2.97, -3.59, 1.71), 566., Some(567.), Some(345.), 20.43),
    entry("Carat", (2.33, -4.1, 2.6), 423., Some(490.), Some(341.), 10.58),
    entry("Cir.L265", (1.29, -1.36, 0.77), 378., Some(400.), Some(276.), 12.6),
    entry("Cirrus 18m", (2.3, -3.46, 1.9), 378., Some(400.), Some(276.), 12.6),
    entry("Cirrus Std.", (1.71, -2.43, 1.46), 300., Some(390.), Some(215.), 10.),
    entry("Club Astir", (2.11, -2.94, 1.68), 372., Some(450.), Some(260.), 12.4),
    entry("Cobra", (2.08, -3.41, 2.07), 348., Some(405.), Some(275.), 11.6),
    entry("DG100", (1.94, -2.9, 1.69), 308., Some(418.), Some(235.), 11.),
    entry("DG1000/18m", (2.51, -4.3, 2.48), 492., Some(750.), Some(411.), 16.72),
    entry("DG1000/20m", (2.66, -4.52, 2.45), 494., Some(750.), Some(415.), 17.53),
    entry("DG1001M", (2.01, -4.16, 2.79), 750., Some(780.), Some(520.), 17.53),
    entry("DG200", (1.15, -1.64, 1.17), 320., Some(450.), Some(238.), 10.),
    entry("DG300", (1.65, -2.55, 1.58), 308., Some(450.), Some(245.), 10.27),
    entry("DG400", (1.13, -1.84, 1.4), 390., Some(480.), Some(306.), 10.),
    entry("DG400/17m", (1.58, -2.72, 1.78), 392., Some(460.), Some(310.), 10.6),
    entry("DG500 TR", (1.51, -2.44, 1.61), 465., Some(615.), Some(390.), 16.6),
    entry("DG500/20m", (1.47, -2.35, 1.51), 528., Some(750.), Some(440.), 17.6),
    entry("DG500/22m", (2.06, -3.38, 1.93), 530., Some(750.), Some(445.), 18.29),
    entry("DG500M", (1.44, -2.27, 1.44), 640., Some(825.), Some(560.), 18.3),
    entry("DG600/15m", (1.31, -2.09, 1.38), 339., Some(525.), Some(257.), 10.95),
    entry("DG600/17m", (1.64, -2.47, 1.43), 336., Some(525.), Some(260.), 11.59),
    entry("DG600/18m", (1.64, -2.47, 1.43), 366., Some(480.), Some(262.), 11.81),
    entry("DG800/15m", (0.92, -1.11, 0.81), 395., Some(525.), Some(323.), 10.68),
    entry("DG800/18m", (1.05, -1.42, 0.93), 402., Some(525.), Some(327.), 11.81),
    entry("DG808", (1.05, -1.42, 0.93), 402., Some(525.), Some(327.), 11.81),
    entry("DG808b", (1.05, -1.42, 0.93), 402., Some(525.), Some(327.), 11.81),
    entry("DG808c", (1.05, -1.42, 0.93), 402., Some(525.), Some(327.), 11.81),
    entry("Diana-2", (1.57, -2.32, 1.33), 242., Some(500.), Some(182.), 8.66),
    entry("Dimona", (2.41, -3.7, 2.58), 457., Some(740.), Some(497.), 15.24),
    entry("Discus", (1.58, -2.46, 1.54), 349., Some(525.), Some(227.), 10.58),
    entry("Discus 2", (2.14, -3.87, 2.38), 335., Some(525.), Some(235.), 10.16),
    entry("Discus 2c", (1.87, -3.15, 1.85), 375., Some(565.), Some(278.), 11.36),
    entry("Discus 2cFES 15m", (2.14, -3.87, 2.38), 335., Some(525.), Some(335.), 10.16),
    entry("Discus 2cFES 18m", (1.87, -3.15, 1.85), 375., Some(565.), Some(345.), 11.36),
    entry("Duo Discus", (1.75, -2.68, 1.54), 492., Some(750.), Some(410.), 16.4),
    entry("EB 28/25m", (0.79, -0.97, 0.64), 616., Some(850.), Some(570.), 15.4),
    entry("EB 28/28m", (0.63, -0.58, 0.42), 660., Some(850.), Some(570.), 16.5),
    entry("EB 29/25.3m", (0.55, -0.63, 0.54), 700., Some(900.), Some(580.), 16.5),
    entry("EB 29/28.3m", (0.62, -0.74, 0.55), 700., Some(900.), Some(580.), 15.4),
    entry("EB 29/29.3m", (0.56, -0.59, 0.47), 700., Some(900.), Some(580.), 16.8),
    entry("EB 29R", (0.63, -0.58, 0.42), 700., Some(850.), Some(614.), 14.9),
    entry("G102 club", (1.69, -2.03, 1.22), 372., Some(380.), Some(248.), 12.4),
    entry("G103 C III SL", (2.54, -4.64, 2.8), 579., Some(710.), Some(500.), 17.52),
    entry("G103 acro", (2.15, -3.23, 1.84), 570., Some(580.), Some(368.), 17.8),
    entry("GP14 SE VELO", (1.24, -1.51, 0.92), 245., Some(425.), Some(175.), 7.),
    entry("Genesis 2", (1.37, -1.98, 1.25), 337., Some(525.), Some(240.), 11.25),
    entry("H205", (1.78, -2.13, 1.2), 294., Some(350.), Some(200.), 9.8),
    entry("H304", (1.15, -1.57, 1.2), 297., Some(450.), Some(235.), 9.9),
    entry("Hornet", (1.7, -2.44, 1.47), 333., Some(420.), Some(227.), 9.8),
    entry("HpH 304M", (1.97, -3.22, 1.78), 400., Some(600.), Some(375.), 11.8),
    entry("HpH 304S", (1.97, -3.22, 1.78), 400., Some(600.), Some(310.), 11.8),
    entry("HpH 304SJ", (1.97, -3.22, 1.78), 400., Some(600.), Some(365.), 11.8),
    entry("HpH 304c", (2.36, -3.81, 2.14), 350., Some(450.), Some(235.), 9.9),
    entry("HpH 304cz", (1.54, -2.42, 1.56), 356., Some(450.), Some(235.), 9.88),
    entry("HpH 304cz17", (1.38, -1.9, 1.16), 346., Some(450.), Some(235.), 9.88),
    entry("HpH 304eS", (1.59, -2.68, 1.68), 425., Some(600.), Some(365.), 11.8),
    entry("IS28B2", (2.57, -4.24, 2.53), 620., Some(590.), Some(400.), 18.24),
    entry("IS29D2", (1.81, -2.31, 1.29), 354., Some(360.), Some(240.), 10.4),
    entry("JS-1", (1.42, -2.62, 1.73), 420., Some(600.), Some(320.), 11.2),
    entry("JS-1-21m", (1.32, -2.24, 1.39), 442., Some(720.), Some(320.), 12.27),
    entry("JS-3-15m", (0.98, -1.85, 1.47), 450., Some(525.), Some(280.), 8.8),
    entry("JS-3-18m", (0.92, -1.66, 1.27), 500., Some(600.), Some(290.), 9.9),
    entry("Jantar 1", (3.01, -5.57, 3.23), 401., Some(515.), Some(295.), 13.38),
    entry("Jantar 2b", (1.57, -2.36, 1.35), 427., Some(476.), Some(356.), 14.25),
    entry("Jantar St2", (2.42, -3.85, 2.18), 320., Some(535.), Some(265.), 10.66),
    entry("Jantar St3", (1.54, -2.32, 1.5), 326., Some(540.), Some(274.), 10.66),
    entry("Janus B", (1.58, -2.44, 1.57), 498., Some(621.), Some(380.), 16.6),
    entry("Janus C", (1.62, -2.67, 1.72), 519., Some(700.), Some(365.), 17.3),
    entry("Janus CM", (1.56, -2.96, 2.1), 675., Some(700.), Some(480.), 17.3),
    entry("Jeans Astir", (1.78, -2.41, 1.51), 372., Some(380.), Some(248.), 12.4),
    entry("KA6e", (2.96, -3.03, 1.35), 270., Some(300.), Some(190.), 12.4),
    entry("Kestrel", (1.66, -2.73, 1.72), 340., Some(400.), Some(260.), 11.6),
    entry("Kestrel 17m", (1.56, -2.53, 1.61), 383., Some(400.), Some(260.), 11.6),
    entry("LAK-13.5m FES", (1.33, -2.13, 1.52), 343., Some(350.), Some(235.), 8.41),
    entry("LAK17-15m", (1.28, -2.21, 1.53), 310., Some(500.), Some(235.), 9.18),
    entry("LAK17-18m", (1.05, -1.42, 0.93), 326., Some(500.), Some(240.), 10.32),
    entry("LAK17B FES", (0.99, -1.65, 1.25), 490., Some(600.), Some(340.), 10.32),
    entry("LAK17CFES-15m", (1.45, -3., 2.26), 427., Some(550.), Some(335.), 9.18),
    entry("LAK17CFES-18m", (1.08, -1.93, 1.43), 444., Some(600.), Some(353.), 10.32),
    entry("LAK17CFES-21m", (1.23, -2.01, 1.3), 455., Some(600.), Some(367.), 11.58),
    entry("LAK17miniFES", (1.83, -3.39, 2.26), 343., Some(350.), Some(213.), 8.41),
    entry("LAK19-15m", (1.62, -2.5, 1.52), 306., Some(480.), Some(235.), 9.06),
    entry("LAK19-18m", (1.24, -1.61, 0.97), 310., Some(480.), Some(240.), 9.8),
    entry("LS 1", (1.81, -2.61, 1.57), 292., Some(390.), Some(230.), 9.74),
    entry("LS 10-s-15", (1.44, -2.35, 1.55), 374., Some(540.), Some(288.), 10.4),
    entry("LS 10-s-18", (1.45, -2.36, 1.49), 378., Some(600.), Some(295.), 11.45),
    entry("LS 1cd", (2.21, -3.08, 1.67), 292., Some(400.), Some(200.), 9.75),
    entry("LS 3", (1.61, -2.5, 1.6), 315., Some(471.), Some(263.), 10.5),
    entry("LS 3 17m", (2.26, -3.79, 2.15), 364., Some(472.), Some(243.), 10.5),
    entry("LS 4", (1.94, -3.35, 2.1), 315., None, None, 10.5),
    entry("LS 6", (1.3, -1.93, 1.31), 346., Some(525.), Some(250.), 10.5),
    entry("LS 6-18", (1.05, -1.42, 0.93), 365., None, None, 11.4),
    entry("LS 7", (1.78, -3.03, 1.93), 292., Some(541.), Some(235.), 9.74),
    entry("LS 8", (2.14, -3.87, 2.38), 336., Some(525.), Some(265.), 10.5),
    entry("LS 8-18", (1.64, -2.47, 1.43), 342., Some(525.), Some(270.), 11.4),
    entry("Mini Nimbus", (1.24, -1.58, 1.03), 325., Some(450.), Some(235.), 9.86),
    entry("Mistral", (2.14, -2.98, 2.03), 327., Some(350.), Some(235.), 10.9),
    entry("Mosquito", (1.13, -1.28, 0.83), 335., Some(450.), Some(235.), 9.85),
    entry("Nimbus 2", (1.41, -2.1, 1.28), 432., Some(650.), Some(355.), 14.39),
    entry("Nimbus 2C", (1.5, -2.25, 1.34), 432., Some(657.), Some(350.), 14.41),
    entry("Nimbus 3", (0.9, -1.15, 0.76), 685., Some(700.), Some(396.), 16.7),
    entry("Nimbus 3D", (0.9, -1.15, 0.76), 741., Some(750.), Some(485.), 16.85),
    entry("Nimbus 4", (1.09, -1.58, 0.96), 732., Some(750.), Some(470.), 17.86),
    entry("Nimbus 4D", (1.05, -1.69, 1.11), 754., Some(750.), Some(515.), 17.96),
    entry("Nimbus 4DM", (1.05, -1.69, 1.11), 754., Some(820.), Some(595.), 17.96),
    entry("Nimbus 4DT", (1.05, -1.69, 1.11), 754., Some(820.), None, 17.96),
    entry("Nimbus 4M", (1.18, -1.75, 1.04), 605., Some(800.), Some(580.), 17.8),
    entry("Nimbus 4T", (1.42, -2.4, 1.44), 607., Some(800.), Some(520.), 17.86),
    entry("PIK 20E", (1.63, -2.98, 2.05), 400., Some(470.), Some(310.), 10.),
    entry("PW5 Smyk", (2.51, -3.22, 1.68), 296., Some(300.), Some(190.), 10.2),
    entry("Pegase", (1.86, -2.66, 1.52), 336., Some(455.), Some(251.), 10.5),
    entry("Phoebus A", (1.28, -1.43, 1.35), 395., Some(350.), Some(220.), 13.16),
    entry("Phoebus B", (1.34, -2.16, 1.39), 327., Some(350.), Some(225.), 13.1),
    entry("Phoebus C", (1.28, -1.43, 0.85), 337., Some(350.), Some(225.), 14.06),
    entry("Phoenix U15", (2.99, -4.77, 2.66), 420., Some(600.), Some(340.), 12.36),
    entry("Piccolo B", (5.57, -7.4, 3.37), 297., Some(297.), Some(180.), 10.6),
    entry("Pilatus B4", (2.34, -2.79, 1.56), 242., Some(350.), Some(320.), 14.),
    entry("Pirat", (2.29, -2.62, 1.36), 414., Some(370.), Some(260.), 13.8),
    entry("Puchacz", (2.08, -2.5, 1.42), 545., Some(570.), Some(368.), 18.16),
    entry("Reiher", (3.37, -3.51, 1.42), 420., Some(450.), Some(336.), 19.),
    entry("S-10-VT", (1.41, -2.25, 1.4), 711., Some(850.), Some(645.), 18.7),
    entry("SF26", (3.03, -4.57, 2.47), 369., Some(310.), Some(190.), 12.3),
    entry("SF27", (3.64, -5.72, 2.89), 390., Some(320.), Some(210.), 13.),
    entry("SF27M", (1.59, -2.28, 1.51), 360., Some(320.), Some(205.), 12.),
    entry("SF34", (1.8, -2.6, 1.65), 444., Some(540.), Some(320.), 14.8),
    entry("SGS 1-26", (3.45, -3.87, 1.9), 444., Some(261.), Some(161.), 14.8),
    entry("SGS 1-26E", (3.43, -4.48, 2.4), 447., Some(318.), Some(202.), 14.9),
    entry("SZD 51-1", (2.76, -3.86, 2.01), 375., Some(355.), Some(242.), 12.51),
    entry("SZD 54-2-17.5", (4.89, -8.41, 4.34), 437., Some(590.), Some(370.), 16.36),
    entry("SZD 54-2-20", (4.9, -8.17, 4.03), 442., Some(615.), Some(375.), 17.29),
    entry("SZD 55", (1.58, -2.46, 1.54), 288., Some(500.), Some(215.), 9.6),
    entry("Silent2 elec.", (1.94, -3.55, 2.41), 297., Some(300.), Some(220.), 9.),
    entry("Silent2 targa", (1.25, -1.46, 0.93), 302., Some(300.), Some(135.), 8.9),
    entry("Sinus", (2.48, -3.85, 2.38), 450., Some(450.), Some(280.), 12.26),
    entry("Slingsby T49B", (2.97, -3.59, 1.71), 566., Some(567.), Some(345.), 20.43),
    entry("Speed Astir", (1.37, -1.94, 1.31), 345., Some(515.), Some(265.), 11.5),
    entry("Std Libelle", (2.14, -3., 1.7), 294., Some(350.), Some(185.), 9.8),
    entry("TST 10 Atlas", (2.48, -3.73, 2.07), 300., Some(320.), Some(201.), 9.85),
    entry("TST 14 Bonus", (2.2, -3.65, 2.26), 467., Some(472.), Some(290.), 12.01),
    entry("Taurus", (2.24, -4.31, 2.78), 358., Some(472.), Some(285.), 12.33),
    entry("Twin Ast.3", (1.92, -2.75, 1.6), 525., Some(600.), Some(390.), 17.5),
    entry("Twin Ast.I", (1.92, -3.16, 2.03), 536., Some(676.), Some(309.), 17.89),
    entry("Twin Ast.II", (1.61, -1.95, 1.2), 534., Some(580.), Some(390.), 17.8),
    entry("UFM-13", (3.61, -5.09, 2.86), 350., Some(472.), Some(265.), 12.2),
    entry("UFM-15", (2.97, -4.29, 2.56), 450., Some(472.), Some(265.), 12.9),
    entry("VSO-10B RG", (2.47, -3.87, 2.17), 347., Some(380.), Some(234.), 12.),
    entry("VSO-10C RG", (3.17, -5.42, 3.1), 347., Some(380.), Some(234.), 12.),
    entry("VT16 Orlik", (3.01, -4.11, 2.13), 335., Some(345.), Some(245.), 12.8),
    entry("VT16 Orlik II", (2.51, -3.11, 1.61), 335., Some(345.), Some(245.), 12.8),
    entry("Ventus", (1.21, -1.78, 1.2), 314., Some(525.), Some(235.), 9.51),
    entry("Ventus 2CM", (0.99, -1.42, 0.98), 430., Some(600.), Some(395.), 11.03),
    entry("Ventus 2CXM", (1.7, -2.98, 1.86), 430., Some(600.), Some(395.), 11.03),
    entry("Ventus 2CXT", (1.19, -1.78, 1.15), 455., Some(600.), Some(355.), 11.03),
    entry("Ventus 3FES", (1.7, -2.98, 1.86), 430., Some(600.), Some(360.), 10.84),
    entry("Ventus 3M", (1.7, -2.98, 1.86), 430., Some(600.), Some(417.), 10.84),
    entry("Ventus 3T", (1.7, -2.98, 1.86), 430., Some(600.), Some(360.), 10.84),
    entry("Ventus A", (1.39, -2.26, 1.51), 313., Some(525.), Some(214.), 9.5),
    entry("Ventus B", (1.63, -2.68, 1.69), 333., Some(525.), Some(225.), 9.51),
    entry("Ventus C", (1.63, -2.68, 1.69), 332., Some(525.), None, 9.5),
    entry("Ventus2/15m", (1.28, -2.21, 1.53), 338., Some(525.), Some(225.), 9.67),
    entry("Ventus2/18m", (1.05, -1.42, 0.93), 375., Some(600.), Some(310.), 11.03),
    entry("VentusA/16m", (1.29, -1.89, 1.2), 330., None, None, 10.),
    entry("VentusB/16m", (1.29, -1.89, 1.2), 330., Some(430.), Some(250.), 10.),
    entry("VentusC/17m", (1.64, -2.47, 1.43), 336., None, None, 10.5),
    entry("VersVS 13.5m", (2.34, -4.05, 2.32), 285., Some(350.), Some(200.), 8.15),
];

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn sorted_by_name() {
        for pair in POLAR_STORE.windows(2) {
            assert!(pair[0].name < pair[1].name, "{:?}", pair[1].name);
        }
    }

    #[test]
    fn entries_are_sane() {
        for entry in POLAR_STORE {
            let polar = entry.glide_polar();

            let min_sink_speed = polar.min_sink_speed().as_kilometers_per_hour();
            let min_sink_rate = polar.min_sink_rate().as_meters_per_second();
            let best_glide_speed = polar.best_glide_speed().as_kilometers_per_hour();
            let best_glide_ratio = polar.best_glide_ratio();

            // Plausibility bands covering everything from the SGS 1-26
            // to an EB 29R. The quadratic model is only valid between
            // roughly minimum sink speed and maximum speed, so the
            // extrapolated vertex can sit below the real stall speed.
            assert!(
                (40. ..110.).contains(&min_sink_speed),
                "{}: min sink speed {min_sink_speed} km/h",
                entry.name
            );
            assert!(
                (0.25..1.2).contains(&min_sink_rate),
                "{}: min sink rate {min_sink_rate} m/s",
                entry.name
            );
            assert!(
                (60. ..130.).contains(&best_glide_speed),
                "{}: best glide speed {best_glide_speed} km/h",
                entry.name
            );
            assert!(
                (20. ..80.).contains(&best_glide_ratio),
                "{}: best glide ratio {best_glide_ratio}",
                entry.name
            );
            assert!(best_glide_speed > min_sink_speed, "{}", entry.name);

            let wing_area = entry.wing_area.as_square_meters();
            assert!(
                (5. ..25.).contains(&wing_area),
                "{}: {wing_area} m²",
                entry.name
            );
        }
    }

    #[test]
    fn matches_stated_performance() {
        // Spot checks against the performance values stated alongside
        // the coefficients in the source list.
        #[track_caller]
        fn check(name: &str, best_ld: f64, at_kmh: f64, min_sink: f64, at_min_sink_kmh: f64) {
            let entry = POLAR_STORE.iter().find(|entry| entry.name == name).unwrap();
            let polar = entry.glide_polar();
            assert_abs_diff_eq!(polar.best_glide_ratio(), best_ld, epsilon = 0.1);
            assert_abs_diff_eq!(
                polar.best_glide_speed().as_kilometers_per_hour(),
                at_kmh,
                epsilon = 1.
            );
            assert_abs_diff_eq!(
                polar.min_sink_rate().as_meters_per_second(),
                min_sink,
                epsilon = 0.001
            );
            assert_abs_diff_eq!(
                polar.min_sink_speed().as_kilometers_per_hour(),
                at_min_sink_kmh,
                epsilon = 1.
            );
        }

        check("Antares 18S", 54.3, 112., 0.54, 99.);
        check("LS 4", 40.4, 104., 0.654, 86.);
        check("ASK 21", 33.9, 98., 0.735, 82.);
        check("Nimbus 4", 59.6, 94., 0.387, 72.);
    }
}
