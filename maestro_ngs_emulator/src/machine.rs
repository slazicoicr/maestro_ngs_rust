type Result<T> = std::result::Result<T, MachineError>;

pub trait Machine {
    fn new() -> Self;
    fn execute(&mut self, exe: &Execute) -> Result<()>;
}

impl Machine for ScicloneG3 {

    fn new() -> Self {
        ScicloneG3 {
            deck_location: None,
            tips_loaded: false,
            tip_volume: 0.0,
        }
    }

    fn execute(&mut self, exe: &Execute) -> Result<()> {
        match exe {
            &Execute::Aspirate { position, volume } => {
                self.move_to(position);
                self.aspirate(volume)?;
            }
            &Execute::Dispense { position, volume } => {
                self.move_to(position);
                self.dispense(volume)?;
            }
            &Execute::EjectTips {position} => {
                self.move_to(position);
                self.eject_tips();
            }
            &Execute::LoadTips { position } => {
                self.move_to(position);
                self.load_tips()?;
            }
            &Execute::Mix { position } => {
                self.move_to(position);
            }
            &Execute::REM { comment: _ } => {}
        }

        Ok(())
    }
}

pub struct ScicloneG3 {
    deck_location: Option<String>,
    tips_loaded: bool,
    tip_volume: f64,
}


impl ScicloneG3 {
    pub fn aspirate(&mut self, volume: f64) -> Result<()> {
        self.assert_tips()?;
        self.tip_volume = self.tip_volume + volume;
        Ok(())
    }

    pub fn dispense(&mut self, volume: Option<f64>) -> Result<()> {
        self.assert_tips()?;
        let volume = match volume {
            Some(v) => v,
            None => self.tip_volume
        };
        if volume > self.tip_volume {
            Err(MachineError::NotEnoughTipVolume)
        } else {
            self.tip_volume = self.tip_volume - volume;
            Ok(())
        }
    }

    pub fn eject_tips(&mut self) {
        self.tips_loaded = false;
        self.tip_volume = 0.0;
    }

    pub fn load_tips(&mut self) -> Result<()> {
        if self.tips_loaded {
            Err(MachineError::TipsAlreadyLoaded)
        } else {
            self.tips_loaded = true;
            Ok(())
        }
    }

    pub fn move_to(&mut self, location: &str) {
        self.deck_location = Some(location.to_string());
    }

    pub fn get_deck_location(&self) -> Option<&String> {
        self.deck_location.as_ref()
    }

    pub fn get_tips_loaded(&self) -> bool {
        self.tips_loaded
    }

    pub fn get_tip_volume(&self) -> f64 {
        self.tip_volume
    }

    fn assert_tips(&self) -> Result<()> {
        if self.tips_loaded {
            Ok(())
        } else {
            Err(MachineError::NeedTips)
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub enum Execute<'a> {
    Aspirate { position: &'a str, volume: f64 },
    // If None volume, dispense all
    Dispense { position: &'a str, volume: Option<f64> },
    EjectTips { position: &'a str },
    LoadTips { position: &'a str },
    Mix { position: &'a str },
    REM { comment: &'a str },
}

#[derive(Debug)]
pub enum MachineError {
    NeedTips,
    NotEnoughTipVolume,
    TipsAlreadyLoaded,
}

impl std::fmt::Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NeedTips => write!(f, "need tips on gantry to do this"),
            Self::NotEnoughTipVolume => write!(f, "not enough volume in tips"),
            Self::TipsAlreadyLoaded => write!(f, "trying to load tips twice"),
        }
    }
}

impl std::error::Error for MachineError {}
