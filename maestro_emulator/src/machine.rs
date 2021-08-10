pub struct Machine {
    deck_location: Option<String>,
    tips_loaded: bool,
    tip_volume: f64,
}

type Result<T> = std::result::Result<T, MachineError>;

impl Machine {
    pub fn new() -> Self {
        Machine {
            deck_location: None,
            tips_loaded: false,
            tip_volume: 0.0,
        }
    }

    pub fn aspirate(&mut self, volume: f64) -> Result<()>{
        self.assert_tips()?;
        self.tip_volume = self.tip_volume + volume;
        Ok(())
    }

    pub fn dispense(&mut self, volume: f64) -> Result<()> {
        self.assert_tips()?;
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