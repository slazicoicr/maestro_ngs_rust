use maestro_application::{Command, SavedApplication, Variable};
use std::collections::HashMap;
use uuid::Uuid;

type Result<T> = std::result::Result<T, EmulatorError>;

pub struct Emulator<'a> {
    saved_app: &'a SavedApplication,
    method_stack: Vec<Uuid>,
    action_executed: Vec<Action<'a>>,
    instruction_stack: Vec<usize>,
    saved_param_stack: Vec<&'a HashMap<Uuid, Variable>>,
}

impl<'a> Emulator<'a> {
    pub fn new(saved_app: &'a SavedApplication) -> Result<Self> {
        let mut emu = Emulator {
            saved_app,
            method_stack: Vec::new(),
            action_executed: Vec::new(),
            instruction_stack: Vec::new(),
            saved_param_stack: Vec::new(),
        };

        let uuid = saved_app.start_method();
        emu.method_stack.push(uuid);

        let saved_param = saved_app
            .parameters_of_method(uuid)
            .ok_or(EmulatorError::UnknownMethod(uuid))?;
        emu.saved_param_stack.push(saved_param);

        emu.instruction_stack.push(0);
        Ok(emu)
    }

    pub fn done(&self) -> bool {
        self.method_stack.len() == 0
    }

    pub fn next(&mut self) -> Result<Option<&Action>> {
        while self.try_finish_method()? {
            continue;
        }

        if self.done() {
            return Ok(None);
        }

        let action = self.build_action()?;
        self.execute_action(&action)?;
        self.action_executed.push(action);
        let line = self
            .instruction_stack
            .last_mut()
            .ok_or(EmulatorError::EmptyInstructionStack)?;
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
            executable: !instr.is_comment,
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
        if action.executable {
            Ok(())
        } else {
            Ok(())
        }
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
        self.saved_param_stack
            .pop()
            .ok_or(EmulatorError::EmptyParameterStack)?;
        Ok(())
    }
}

pub struct Action<'a> {
    pub method: Uuid,
    pub line: usize,
    pub executable: bool,
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
