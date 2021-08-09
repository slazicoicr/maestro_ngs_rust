pub struct Machine {
    pub(super) deck_location: Option<String>,
    pub(super) tips_loaded: bool,
}

type Result<T> = std::result::Result<T, MachineError>;

impl Machine {
    pub fn new() -> Self {
        Machine {
            deck_location: None,
            tips_loaded: false,
        }
    }

    pub fn move_to(&mut self, location: &str) {
        self.deck_location = Some(location.to_string());
    }

    pub fn load_tips(&mut self) -> Result<()> {
        if self.tips_loaded {
            Err(MachineError::TipsAlreadyLoaded)
        } else {
            self.tips_loaded = true;
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum MachineError {
    TipsAlreadyLoaded,
}

impl std::fmt::Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TipsAlreadyLoaded => write!(f, "trying to load tips twice"),
        }
    }
}

impl std::error::Error for MachineError {}