use maestro_application::{Command, SavedApplication, Variable};
use std::{collections::HashMap, hash::Hash};
use uuid::Uuid;

type Result<T> = std::result::Result<T, EmulatorError>;

pub struct Emulator<'a> {
    saved_app: &'a SavedApplication,
    global_variables: HashMap<Uuid, Variable>,
    method_stack: Vec<Uuid>,
    action_executed: Vec<Action<'a>>,
    instruction_stack: Vec<usize>,
    param_stack: Vec<HashMap<Uuid, Variable>>,
    local_variables: HashMap<Uuid, HashMap<Uuid, Variable>>
}

impl<'a> Emulator<'a> {
    pub fn new(saved_app: &'a SavedApplication) -> Result<Self> {
        let mut emu = Emulator {
            saved_app,
            global_variables: saved_app.global_variables().clone(),
            method_stack: Vec::new(),
            action_executed: Vec::new(),
            instruction_stack: Vec::new(),
            param_stack: Vec::new(),
            local_variables: HashMap::new(),
        };

        let uuid = saved_app.start_method();
        emu.method_stack.push(uuid);

        let saved_param = saved_app
            .parameters_of_method(uuid)
            .cloned()
            .ok_or(EmulatorError::UnknownMethod(uuid))?;
        emu.param_stack.push(saved_param);

        for &uuid in saved_app.ids_methods() {
            let local = saved_app.local_variables_of_method(uuid).ok_or(EmulatorError::UnknownMethod(uuid))?;
            emu.local_variables.insert(uuid, local.clone());
        }

        emu.instruction_stack.push(0);
        Ok(emu)
    }

    pub fn done(&self) -> bool {
        self.method_stack.len() == 0
    }

    pub fn next(&mut self) -> Result<Option<&Action>> {
        // Multiple methods may be finished. If a method A is last instruction of method B.
        while self.try_finish_method()? {
            continue;
        }

        if self.done() {
            return Ok(None);
        }

        let action = self.build_action()?;
        self.execute_action(&action)?;
        let line = self
            .instruction_stack
            .last_mut()
            .ok_or(EmulatorError::EmptyInstructionStack)?;
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
            skip: !instr.is_comment,
            execute: exe,
        })
    }

    fn build_execute(&self, command: &'a Command) -> Result<Execute<'a>> {
        match command {
            Command::REM { comment } => Ok(Execute::REM { comment }),
            _ => panic!("Unknown command {:?}", command),
        }
    }

    fn execute_action(&mut self, action: &Action) -> Result<()> {
        if action.skip {
            return Ok(())
        }

        match action.execute {
            Execute::REM{comment: _} => {},
        }

        Ok(())
        
    }

    fn get_current_instruction(&self) -> Result<usize> {
        self.instruction_stack
            .last()
            .cloned()
            .ok_or(EmulatorError::EmptyMethodStack)
    }

    fn get_current_method(&self) -> Result<Uuid> {
        self.method_stack
            .last()
            .cloned()
            .ok_or(EmulatorError::EmptyMethodStack)
    }

    fn try_finish_method(&mut self) -> Result<bool> {
        if let Some(&method_id) = self.method_stack.last() {
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
        self.method_stack
            .pop()
            .ok_or(EmulatorError::EmptyMethodStack)?;
        self.instruction_stack
            .pop()
            .ok_or(EmulatorError::EmptyInstructionStack)?;
        self.param_stack
            .pop()
            .ok_or(EmulatorError::EmptyParameterStack)?;
        Ok(())
    }
}

pub struct Action<'a> {
    pub method: Uuid,
    pub line: usize,
    pub skip: bool,
    pub execute: Execute<'a>,
}

pub enum Execute<'a> {
    REM { comment: &'a str },
}

#[derive(Debug)]
pub enum EmulatorError {
    EmptyInstructionStack,
    EmptyMethodStack,
    EmptyParameterStack,
    UnknownMethod(Uuid),
    UnknownInstruction(Uuid, usize),
}

impl std::fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInstructionStack => {
                write!(f, "cannot get next instruction as stack is empty")
            }
            Self::EmptyMethodStack => write!(f, "cannot get next method as stack is empty"),
            Self::EmptyParameterStack => write!(f, "cannot get parameters as stack is empty"),
            Self::UnknownMethod(uuid) => write!(f, "unknown method ({})", uuid),
            Self::UnknownInstruction(uuid, line) => write!(
                f,
                "instruction line {} does not exist for method {}",
                line, uuid
            ),
        }
    }
}

impl std::error::Error for EmulatorError {}

#[cfg(test)]
mod tests {
    use super::*;
    use maestro_application::Loader; 

    fn load_empty_app() -> String {
        let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/Application_Empty.eap");

        std::fs::read_to_string(d).unwrap()
    }

    fn load_complex_app() -> String {
        let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/test/Application_Complex.eap");

        std::fs::read_to_string(d).unwrap()
    }

    #[test]
    fn emulate_empty_app(){
        let app = Loader::new(&load_empty_app()).build_application();
        let mut emu = Emulator::new(&app).unwrap();
        let uuid = "3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap();
        assert_eq!(emu.method_stack.len(), 1);
        assert_eq!(emu.method_stack[0], uuid);
        assert_eq!(emu.global_variables.len(), 0);
        assert_eq!(emu.param_stack.len(), 1);
        assert_eq!(emu.param_stack[0].len(), 0);
        assert_eq!(emu.local_variables.len(), 1);
        assert_eq!(emu.local_variables.get(&uuid).unwrap().len(), 0);

        let step = emu.next().unwrap();
        assert!(step.is_none());
        assert!(emu.done());
    }
}
