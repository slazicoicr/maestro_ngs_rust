mod machine;

use machine::{Execute, Machine, MachineError, ScicloneG3};
use maestro_ngs_application::{
    Command, InstructionValue, Layout, LoadEjectTipsHead, PositionHead, SavedApplication, Variable,
    VariableValue,
};
use serde::{self, ser::SerializeStruct};
use std::collections::HashMap;
use uuid::Uuid;

type Result<T> = std::result::Result<T, EmulatorError>;
type ScicloneG3Emulator<'a> = Emulator<'a, ScicloneG3>;

pub struct Emulator<'a, M: Machine> {
    saved_app: &'a SavedApplication,
    machine: M,
    action_executed: Vec<Action<'a>>,
    global_variables: HashMap<Uuid, Variable>,
    layouts: &'a HashMap<Uuid, Layout>,
    local_variables: HashMap<Uuid, HashMap<Uuid, Variable>>,
    stack_methods: Vec<Uuid>,
    stack_instructions: Vec<usize>,
    stack_params: Vec<HashMap<Uuid, Variable>>,
    stack_layout: Vec<Uuid>,
}

impl<'a, M: Machine> Emulator<'a, M> {
    pub fn new(saved_app: &'a SavedApplication) -> Result<Self> {
        let mut emu = Emulator {
            saved_app,
            machine: M::new(),
            action_executed: Vec::new(),
            global_variables: saved_app.global_variables().clone(),
            layouts: saved_app.layouts(),
            stack_methods: Vec::new(),
            stack_instructions: Vec::new(),
            stack_params: Vec::new(),
            local_variables: HashMap::new(),
            stack_layout: Vec::new(),
        };

        let uuid = saved_app.start_method();

        for &uuid in emu.saved_app.ids_methods() {
            let local = emu
                .saved_app
                .local_variables_of_method(uuid)
                .ok_or(EmulatorError::UnknownMethod(uuid))?;
            emu.local_variables.insert(uuid, local.clone());
        }

        Emulator::push_method(&mut emu, uuid)?;
        Ok(emu)
    }

    fn push_method(emu: &mut Self, uuid: Uuid) -> Result<()> {
        emu.stack_methods.push(uuid);

        let layout_uuid = emu
            .saved_app
            .layout_of_method(uuid)
            .ok_or(EmulatorError::UnknownMethod(uuid))?;
        emu.stack_layout.push(layout_uuid);

        let saved_param = emu
            .saved_app
            .parameters_of_method(uuid)
            .cloned()
            .ok_or(EmulatorError::UnknownMethod(uuid))?;
        emu.stack_params.push(saved_param);

        emu.stack_instructions.push(0);
        Ok(())
    }

    pub fn done(&self) -> bool {
        self.stack_methods.len() == 0
    }

    pub fn next(&mut self) -> Result<Option<&Action>> {
        // Multiple methods may be finished. If a method A is last instruction of Main method.
        while self.try_finish_method()? {
            continue;
        }

        if self.done() {
            return Ok(None);
        }

        let action = self.build_action()?;
        self.execute_action(&action)?;
        let line = self
            .stack_instructions
            .last_mut()
            .ok_or(EmulatorError::EmptyStack)?;
        self.action_executed.push(action);
        *line += 1;
        Ok(Some(self.action_executed.last().unwrap()))
    }

    fn build_action(&self) -> Result<Action<'a>> {
        let method_id = self.get_current_method()?;
        let current_line = self.get_current_instruction()?;

        let instr = if !self.saved_app.has_method(method_id) {
            Err(EmulatorError::UnknownMethod(method_id))
        } else {
            self.saved_app
                .instruction(method_id, current_line)
                .ok_or(EmulatorError::UnknownInstruction(method_id, current_line))
        }?;
        let exe = self.build_execute(&instr.command)?;
        Ok(Action {
            method: method_id,
            line: current_line,
            skip: instr.is_comment,
            execute: exe,
        })
    }

    fn build_execute(&self, command: &'a Command) -> Result<Execute<'a>> {
        match command {
            Command::Aspirate {
                position_head,
                volume,
            } => {
                let position = self.get_position_positionhead(position_head)?;
                let vol = self.get_instruction_value_float(volume)?;
                Ok(Execute::Aspirate {
                    position,
                    volume: vol,
                })
            }
            Command::Dispense {
                position_head,
                volume,
                dispense_all,
            } => {
                let position = self.get_position_positionhead(position_head)?;
                let vol = if *dispense_all {
                    None
                } else {
                    Some(self.get_instruction_value_float(volume)?)
                };
                Ok(Execute::Dispense {
                    position,
                    volume: vol,
                })
            }
            Command::EjectTips {
                load_eject_tips_head,
            } => {
                let position = self.get_position_loadeject_tip_head(load_eject_tips_head)?;
                Ok(Execute::EjectTips { position })
            }
            Command::LoadTips {
                load_eject_tips_head,
            } => {
                let position = self.get_position_loadeject_tip_head(load_eject_tips_head)?;
                Ok(Execute::LoadTips { position })
            }
            Command::Mix { position_head } => {
                let position = self.get_position_positionhead(position_head)?;
                Ok(Execute::Mix { position })
            }
            Command::REM { comment } => Ok(Execute::REM { comment }),
            _ => panic!("Unknown command {:?}", command),
        }
    }

    fn execute_action(&mut self, action: &Action) -> Result<()> {
        if action.skip {
            return Ok(());
        }

        self.machine.execute(&action.execute)?;
        Ok(())
    }

    fn get_current_instruction(&self) -> Result<usize> {
        self.stack_instructions
            .last()
            .cloned()
            .ok_or(EmulatorError::EmptyStack)
    }

    fn get_current_layout(&self) -> Result<Uuid> {
        self.stack_layout
            .last()
            .cloned()
            .ok_or(EmulatorError::EmptyStack)
    }

    fn get_current_layout_position(&self, position_uuid: Uuid) -> Result<&'a String> {
        let uuid = self.get_current_layout()?;
        let layout = self
            .layouts
            .get(&uuid)
            .ok_or(EmulatorError::UnknownLayout(uuid))?;
        let pos = layout
            .position(position_uuid)
            .ok_or(EmulatorError::UnknownLayoutPosition(position_uuid))?;
        Ok(pos)
    }

    fn get_current_method(&self) -> Result<Uuid> {
        self.stack_methods
            .last()
            .cloned()
            .ok_or(EmulatorError::EmptyStack)
    }

    fn get_instruction_value_float(&self, inst: &'a InstructionValue) -> Result<f64> {
        if inst.variable.is_some() {
            panic!("Can't deal with variable in InstructionValue {:?}", inst)
        }

        match inst.direct {
            VariableValue::Float(f) => Ok(f),
            _ => Err(EmulatorError::UnexpectedType),
        }
    }

    fn get_position_positionhead(&self, pos: &'a PositionHead) -> Result<&'a String> {
        match pos.deck_parameter {
            Some(uuid) => Ok(self.get_current_layout_position(uuid)?),
            None => panic!(
                "Did not expect InstructionValue for {:?}",
                pos.deck_location
            ),
        }
    }

    fn get_position_loadeject_tip_head(&self, pos: &'a LoadEjectTipsHead) -> Result<&'a String> {
        match pos.deck_parameter {
            Some(uuid) => Ok(self.get_current_layout_position(uuid)?),
            None => panic!(
                "Did not expect InstructionValue for {:?}",
                pos.deck_location
            ),
        }
    }

    fn try_finish_method(&mut self) -> Result<bool> {
        if let Some(&method_id) = self.stack_methods.last() {
            let current_instr = self.get_current_instruction()?;
            let instr_count = self
                .saved_app
                .instruction_count(method_id)
                .ok_or(EmulatorError::UnknownMethod(method_id))?;
            if current_instr >= instr_count {
                self.pop_method()?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    fn pop_method(&mut self) -> Result<()> {
        self.stack_methods.pop().ok_or(EmulatorError::EmptyStack)?;
        self.stack_instructions
            .pop()
            .ok_or(EmulatorError::EmptyStack)?;
        self.stack_params.pop().ok_or(EmulatorError::EmptyStack)?;
        self.stack_layout.pop().ok_or(EmulatorError::EmptyStack)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Action<'a> {
    pub method: Uuid,
    pub line: usize,
    pub skip: bool,
    pub execute: Execute<'a>,
}

impl<'a> serde::Serialize for Action<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Action", 4)?;
        state.serialize_field("method", &self.method.to_string())?;
        state.serialize_field("line", &self.line)?;
        state.serialize_field("skip", &self.skip)?;
        state.serialize_field("execute", &self.execute)?;
        state.end()
    }
}

#[derive(Debug)]
pub enum EmulatorError {
    EmptyStack,
    MachineError(MachineError),
    UnexpectedType,
    UnknownLayout(Uuid),
    UnknownLayoutPosition(Uuid),
    UnknownMethod(Uuid),
    UnknownInstruction(Uuid, usize),
}

impl std::fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyStack => write!(f, "emulator stack is unexpectendly empty"),
            Self::MachineError(m) => m.fmt(f),
            Self::UnexpectedType => write!(f, "unexpected variable type"),
            Self::UnknownLayout(uuid) => write!(f, "unknown layout ({})", uuid),
            Self::UnknownLayoutPosition(uuid) => {
                write!(f, "unknown layout position variable ({})", uuid)
            }
            Self::UnknownInstruction(uuid, line) => write!(
                f,
                "instruction line {} does not exist for method {}",
                line, uuid
            ),
            Self::UnknownMethod(uuid) => write!(f, "unknown method ({})", uuid),
        }
    }
}

impl std::error::Error for EmulatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EmptyStack => None,
            Self::MachineError(m) => Some(m),
            Self::UnexpectedType => None,
            Self::UnknownLayout(_) => None,
            Self::UnknownLayoutPosition(_) => None,
            Self::UnknownInstruction(_, _) => None,
            Self::UnknownMethod(_) => None,
        }
    }
}

impl From<MachineError> for EmulatorError {
    fn from(error: MachineError) -> Self {
        EmulatorError::MachineError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maestro_ngs_application::Loader;

    fn load_empty_app() -> String {
        let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/Application_Empty.eap");

        std::fs::read_to_string(d).unwrap()
    }

    fn load_pipette_and_mix_app() -> String {
        let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/Pipette_and_Mix.eap");

        std::fs::read_to_string(d).unwrap()
    }

    #[test]
    fn emulate_empty_app() {
        let app = Loader::new(&load_empty_app()).build_application();
        let mut emu = ScicloneG3Emulator::new(&app).unwrap();
        let uuid = "3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap();
        assert_eq!(emu.stack_methods.len(), 1);
        assert_eq!(emu.stack_methods[0], uuid);
        assert_eq!(emu.global_variables.len(), 0);
        assert_eq!(emu.stack_params.len(), 1);
        assert_eq!(emu.stack_params[0].len(), 0);
        assert_eq!(emu.local_variables.len(), 1);
        assert_eq!(emu.local_variables.get(&uuid).unwrap().len(), 0);

        let step = emu.next().unwrap();
        assert!(step.is_none());
        assert!(emu.done());
    }

    #[test]
    fn emulate_pipette_and_mix_app() {
        let app = Loader::new(&load_pipette_and_mix_app()).build_application();
        let mut emu = ScicloneG3Emulator::new(&app).unwrap();

        // Load tips
        let mut step = emu.next().unwrap();
        assert!(step.is_some());
        assert_eq!(emu.machine.get_deck_location(), Some(&"C3".to_string()));
        assert!(emu.machine.get_tips_loaded());

        // Aspirate 100 uL
        step = emu.next().unwrap();
        assert!(step.is_some());
        assert_eq!(emu.machine.get_deck_location(), Some(&"C4".to_string()));
        assert_eq!(emu.machine.get_tip_volume(), 100.0);

        // Dispense
        step = emu.next().unwrap();
        assert!(step.is_some());
        assert_eq!(emu.machine.get_deck_location(), Some(&"B4".to_string()));
        assert_eq!(emu.machine.get_tip_volume(), 0.0);

        // Mix
        step = emu.next().unwrap();
        assert!(step.is_some());
        assert_eq!(emu.machine.get_deck_location(), Some(&"B4".to_string()));

        step = emu.next().unwrap();
        assert!(step.is_some());
        assert_eq!(emu.machine.get_deck_location(), Some(&"D5".to_string()));
        assert!(!emu.machine.get_tips_loaded());

        step = emu.next().unwrap();
        assert!(step.is_none());
        assert!(emu.done());
    }
}
