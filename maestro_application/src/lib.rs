use roxmltree::{Document, Node};
use std::{collections::HashMap, hash::Hash};
use uuid::Uuid;

const APP: &str = "Application";
const APP_BUILD: &str = "ExportedApplicationBuild";
const APP_VERSION: &str = "ExportedApplicationVersion";
const GLOBAL_VAR_POOL: &str = "GlobalVariablesPool";
const INSTR_COMPARATOR: &str = "Comparator";
const INSTR_COUNT: &str = "InstructionsCount";
const INSTR_TEST_TYPE: &str = "DataTypeOfTest";
const INSTR_DESIG: &str = "InstructionDesignation";
const INSTR_DIRECT_VALUE: &str = "_DirectValue";
const INSTR_IS_COMMENT: &str = "IsComment";
const INSTR_VARIABLE: &str = "_Variable";
const LAYOUT_ID: &str = "LayoutID";
const LAYOUTS: &str = "Layouts";
const LAYOUTS_COUNT: &str = "LayoutsCount";
const LOCAL_VAR_POOL: &str = "LocalVariablesPool";
const METHODS: &str = "Methods";
const METHODS_COUNT: &str = "MethodsCount";
const METHOD_DESIG: &str = "MethodDesignation";
const PARAM_TYPE: &str = "ParameterType";
const PARAM_ID: &str = "ForParameter";
const PARAMS: &str = "Parameters";
const PROGRAM_ID: &str = "ProgramID";
const START_METHOD: &str = "StartupMethod";
const VAR_CONSUMABLE: &str = "IDAccOrCon";
const VAR_COUNT: &str = "VariablesCount";
const VAR_DESIG: &str = "VariableDesignation";
const VAR_ID: &str = "VariableID";
const VAR_NUMBER_STACKED: &str = "NumberOfStackedConsumables";
const VAR_POOL_DESIG: &str = "VariablesPoolDesignation";
const VAR_POOL_ID: &str = "VariablesPoolID";
const VAR_THIS_DESIG: &str = "ThisDesignation";
const VAR_VALUE: &str = "Value";
const VAR_TYPE: &str = "VariableType";

pub struct Loader<'a> {
    raw: Document<'a>,
    version: f64,
    build: u32,
}

impl<'a> Loader<'a> {
    pub fn new(instruction_text: &'a str) -> Self {
        let raw = Document::parse(instruction_text).unwrap();
        let version = get_float_text(&raw.root(), APP_VERSION);
        let build = get_int_text(&raw.root(), APP_BUILD);
        Loader {
            raw,
            version,
            build,
        }
    }

    pub fn input_text(&self) -> &str {
        self.raw.input_text()
    }

    pub fn version(&self) -> f64 {
        self.version
    }

    pub fn build(&self) -> u32 {
        self.build
    }

    pub fn build_application(&self) -> SavedApplication {
        let app = self
            .raw
            .descendants()
            .find(|n| n.has_tag_name(APP))
            .unwrap();
        let flat_fields = text_only_children(&app);

        let mut result = SavedApplication {
            start_method: flat_fields.get(START_METHOD).unwrap().parse().unwrap(),
            global_variables: HashMap::new(),
            layouts: HashMap::new(),
            methods: HashMap::new(),
        };

        for c in app.children() {
            if c.has_tag_name(GLOBAL_VAR_POOL) {
                let global_var = Self::build_variables_pool(&c.first_element_child().unwrap());
                result.set_global_variables(global_var);
            } else if c.has_tag_name(LAYOUTS) {
                for layouts in c
                    .children()
                    .filter(|n| n.is_element() && !n.has_tag_name(LAYOUTS_COUNT))
                {
                    let layout_var = Self::build_layout(&layouts.first_element_child().unwrap());
                    result.add_layout(layout_var);
                }
            } else if c.has_tag_name(METHODS) {
                for method_nodes in c
                    .children()
                    .filter(|n| n.is_element() && !n.has_tag_name(METHODS_COUNT))
                {
                    let method = Self::build_method(&method_nodes);
                    result.add_method(method);
                }
            }
        }
        result
    }

    fn build_variable(node: &Node) -> Variable {
        let variable_fields = text_only_children(node);
        let val_str = variable_fields.get(VAR_VALUE).unwrap();
        let value = match *variable_fields.get(VAR_TYPE).unwrap() {
            "2" => Some(VariableValue::Float(val_str.parse().unwrap())),
            "3" => Some(VariableValue::String(val_str.to_string())),
            "4" => {
                let b = Self::build_bool(&val_str);
                Some(VariableValue::Bool(b))
            }
            "7" => Some(VariableValue::Seconds(val_str.parse().unwrap())),
            _ => None,
        };
        Variable {
            designation: variable_fields.get(VAR_DESIG).unwrap().to_string(),
            id: variable_fields.get(VAR_ID).unwrap().parse().unwrap(),
            value: value.unwrap(),
        }
    }

    fn build_parameter(node: &Node) -> Parameter {
        let variable_fields = text_only_children(node);
        let uuid_str = variable_fields.get("ForParameter").unwrap();
        let val_type_str = variable_fields.get("ParameterType").unwrap();
        let val_type = match *val_type_str {
            "2" => VariableType::Float,
            "3" => VariableType::String,
            "4" => VariableType::Bool,
            "7" => VariableType::Seconds,
            _ => panic!("Unknown parameter type {}", val_type_str),
        };
        let val = Self::build_instruction_value(&node, val_type);
        Parameter {
            id: uuid_str.parse().unwrap(),
            value: val,
        }
    }

    fn build_variables_pool(node: &Node) -> VariablesPool {
        let global_fields = text_only_children(node);
        let var_count = node
            .descendants()
            .find(|n| n.has_tag_name(VAR_COUNT))
            .unwrap();
        let mut var_map = HashMap::new();

        // The sibling element iterator includes itself, so skip it
        for n in var_count.next_siblings().skip(1).filter(|n| n.is_element()) {
            let var = Self::build_variable(&n);
            var_map.insert(var.id, var);
        }

        VariablesPool {
            designation: global_fields.get(VAR_POOL_DESIG).unwrap().parse().unwrap(),
            id: global_fields.get(VAR_POOL_ID).unwrap().parse().unwrap(),
            variables: var_map,
        }
    }

    fn build_location(node: &Node) -> Location {
        let variable_fields = text_only_children(node);
        Location {
            id: variable_fields.get(VAR_ID).unwrap().parse().unwrap(),
            position: variable_fields.get(VAR_DESIG).unwrap().to_string(),
            number_stacked: variable_fields
                .get(VAR_NUMBER_STACKED)
                .unwrap()
                .parse()
                .unwrap(),
            designation: variable_fields.get(VAR_THIS_DESIG).unwrap().to_string(),
            consumable: variable_fields
                .get(VAR_CONSUMABLE)
                .unwrap()
                .parse()
                .unwrap(),
        }
    }

    fn build_layout(node: &Node) -> Layout {
        let global_fields = text_only_children(node);
        let var_count = node
            .descendants()
            .find(|n| n.has_tag_name(VAR_COUNT))
            .unwrap();
        let mut var_map = HashMap::new();

        // The sibling element iterator includes itself, so skip it
        for n in var_count.next_siblings().skip(1).filter(|n| n.is_element()) {
            let var = Self::build_location(&n);
            var_map.insert(var.id, var);
        }

        Layout {
            designation: global_fields.get(VAR_POOL_DESIG).unwrap().parse().unwrap(),
            id: global_fields.get(VAR_POOL_ID).unwrap().parse().unwrap(),
            positions: var_map,
        }
    }

    fn build_method(node: &Node) -> Method {
        let method_fields = text_only_children(node);
        let mut local_var: Option<VariablesPool> = None;
        let mut params: Option<VariablesPool> = None;
        let mut instructions = Vec::new();
        let mut reached_instructions = false;
        for c in node.children() {
            if reached_instructions && c.is_element() {
                instructions.push(Self::build_instruction(&c));
            } else if c.has_tag_name(LOCAL_VAR_POOL) {
                local_var = Some(Self::build_variables_pool(
                    &c.first_element_child().unwrap(),
                ));
            } else if c.has_tag_name(PARAMS) {
                params = Some(Self::build_variables_pool(
                    &c.first_element_child().unwrap(),
                ));
            } else if c.has_tag_name(INSTR_COUNT) {
                reached_instructions = true;
            }
        }
        Method {
            designation: method_fields.get(METHOD_DESIG).unwrap().parse().unwrap(),
            id: method_fields.get(PROGRAM_ID).unwrap().parse().unwrap(),
            layout_id: method_fields.get(LAYOUT_ID).unwrap().parse().unwrap(),
            local_variables_pool: local_var.unwrap(),
            parameters: params.unwrap(),
            instructions,
        }
    }

    fn build_instruction(node: &Node) -> Instruction {
        let instr_fields = text_only_children(node);
        let instr = instr_fields.get(INSTR_DESIG).unwrap();
        let is_comment_str = instr_fields.get(INSTR_IS_COMMENT).unwrap();
        let is_comment = Self::build_bool(*is_comment_str);
        let command = match *instr {
            "Absolute Move" => Command::AbsoluteMove,
            "Application Exit" => Command::ApplicationExit,
            "Aspirate" => Self::build_instruction_aspirate(&node),
            "Begin Loop" => Self::build_instruction_begin_loop(&node),
            "CloseWorkbook" => Command::CloseWorkbook,
            "Dispense" => Self::build_instruction_dispense(&node),
            "End If" => Command::EndIf,
            "End Loop" => Command::EndLoop,
            "End While" => Command::EndWhile,
            "Eject Tips" => Self::build_instruction_eject_tips(&node),
            "Execute VSTA Macro" => Self::build_instruction_execute_vsta_macro(&node),
            "Get Current Position Relative to Reference" => {
                Command::GetCurrentPositionRelativeToReference
            }
            "Head Position" => Self::build_instruction_head_position(&node),
            "Home" => Self::build_instruction_home(&node),
            "Home P Axis" => Command::HomePAxis,
            "If..Then" => Self::build_instruction_if_then(&node),
            "Initialize" => Command::Initialize,
            "Initialize System" => Command::InitializeSystem,
            "Load Tips" => Self::build_instruction_load_tips(&node),
            "Math Operation" => Self::build_instruction_math_operation(&node),
            "Mix" => Self::build_instruction_mix(&node),
            "Move Material" => Self::build_instruction_move_material(&node),
            "OpenWorkbook" => Command::OpenWorkbook,
            "P Axis Set Position" => Command::PAxisSetPosition,
            "Pick" => Self::build_instruction_pick(&node),
            "Place" => Self::build_instruction_place(&node),
            "Relative Move" => Command::RelativeMove,
            "REM" => Self::build_instruction_rem(&node),
            "RunMacro" => Command::RunMacro,
            "Run Method" => Self::build_instruction_run_method(&node),
            "Run Shaker For Time" => Self::build_instruction_run_shaker_for_time(&node),
            "Set Leg Light Intensity" => Self::build_instruction_set_light_intensity(&node),
            "Set Speed" => Self::build_instruction_set_speed(&node),
            "Set Temperature" => Self::build_instruction_set_temperature(&node),
            "Set Travel Height" => Command::SetTravelHeight,
            "SetWorkingDirectory" => Command::SetWorkingDirectory,
            "Shaker On/Off" => Self::build_instruction_temperature_on_off(&node),
            "Show Dialog" => Self::build_show_dialog(&node),
            "Start Timer" => Command::StartTime,
            "Stop Timer" => Command::StopTimer,
            "String Operation" => Command::StringOperation,
            "Temperature On/Off" => Self::build_instruction_shaker_on_off(&node),
            "UnGrip" => Command::Ungrip,
            "Vertical Position" => Command::VerticalPosition,
            "While Loop" => Self::build_instruction_while_loop(&node),
            _ => panic!("Unknown command {}", instr),
        };
        Instruction {
            is_comment,
            command,
        }
    }

    fn build_operator(op: &str) -> Operator {
        match op {
            "(Assignment)" => Operator::Assign,
            "-" => Operator::Minus,
            "+" => Operator::Plus,
            _ => panic!("Unknown math operator {}", op),
        }
    }

    fn build_test_variable_type(var: &str) -> VariableType {
        match var {
            "0" => VariableType::String,
            "1" => VariableType::Float,
            "2" => VariableType::Bool,
            _ => panic!("Unknown test variable type {}", var),
        }
    }

    fn build_comparator(comp: &str) -> Comparator {
        match comp {
            "Equals" => Comparator::Equals,
            "Greater than" => Comparator::GreaterThan,
            "Greater than or equal to" => Comparator::GreaterThanOrEqual,
            "Less than" => Comparator::LessThan,
            "Less than or equal to" => Comparator::LessThanOrEqual,
            _ => panic!("Unknown comparator {}", comp),
        }
    }

    fn build_position_head(node: &Node) -> PositionHead {
        let uuid_str = node
            .descendants()
            .find(|n| n.has_tag_name("DeckVariableID"))
            .unwrap()
            .text()
            .unwrap();
        let mut deck_parameter = None;
        if uuid_str != "[[[[---NONE---]]]]" {
            deck_parameter = Some(uuid_str.parse().unwrap());
        }
        let var_node = node
            .descendants()
            .find(|n| n.has_tag_name("DeckLocation"))
            .unwrap();
        let deck_location = Self::build_instruction_value(&var_node, VariableType::String);

        let z_offset_node = var_node
            .next_siblings()
            .find(|n| n.has_tag_name("ZPosOffset"))
            .unwrap();
        let z_offset = Self::build_instruction_value(&z_offset_node, VariableType::Float);
        PositionHead {
            deck_parameter,
            deck_location,
            z_offset,
        }
    }

    fn build_load_eject_tips_head(node: &Node) -> LoadEjectTipsHead {
        let uuid_str = node
            .descendants()
            .find(|n| n.has_tag_name("DeckVariableID"))
            .unwrap()
            .text()
            .unwrap();
        let mut deck_parameter = None;
        if uuid_str != "[[[[---NONE---]]]]" {
            deck_parameter = Some(uuid_str.parse().unwrap());
        }
        let var_node = node
            .descendants()
            .find(|n| n.has_tag_name("DeckLocation"))
            .unwrap();
        let deck_location = Self::build_instruction_value(&var_node, VariableType::String);
        LoadEjectTipsHead {
            deck_parameter,
            deck_location,
        }
    }

    fn build_bool(s: &str) -> bool {
        if s == "0" {
            false
        } else {
            true
        }
    }

    fn build_instruction_aspirate(node: &Node) -> Command {
        let position_node = node
            .descendants()
            .find(|n| n.has_tag_name("HeadPosInstr"))
            .unwrap();
        let position = Self::build_position_head(&position_node);
        let vol_node = position_node
            .next_siblings()
            .find(|n| n.has_tag_name("VarVolume"))
            .unwrap();
        let vol = Self::build_instruction_value(&vol_node, VariableType::Float);
        Command::Aspirate {
            position_head: position,
            volume: vol,
        }
    }

    fn build_instruction_begin_loop(node: &Node) -> Command {
        let index_node = node
            .descendants()
            .find(|n| n.has_tag_name("LoopIndexParam"))
            .unwrap();
        let index = Self::build_instruction_value(&index_node, VariableType::Int);
        let from_node = index_node
            .next_siblings()
            .find(|n| n.has_tag_name("LoopFromParam"))
            .unwrap();
        let from = Self::build_instruction_value(&from_node, VariableType::Int);
        let to_node = from_node
            .next_siblings()
            .find(|n| n.has_tag_name("LoopToParam"))
            .unwrap();
        let to = Self::build_instruction_value(&to_node, VariableType::Int);
        let steps_node = to_node
            .next_siblings()
            .find(|n| n.has_tag_name("LoopStepParam"))
            .unwrap();
        let steps = Self::build_instruction_value(&steps_node, VariableType::Int);
        Command::BeginLoop {
            index,
            from,
            to,
            steps,
        }
    }

    fn build_instruction_dispense(node: &Node) -> Command {
        let dcc_control_node = node
            .descendants()
            .find(|n| n.has_tag_name("DCCControl"))
            .unwrap();
        if dcc_control_node.text().unwrap() == "Sciclone" {
            let all_node = node
                .descendants()
                .find(|n| n.has_tag_name("DispenseAll"))
                .unwrap();
            let dispense_all = Self::build_bool(all_node.text().unwrap());
            let head_node = all_node
                .next_siblings()
                .find(|n| n.has_tag_name("HeadPosInstr"))
                .unwrap();
            let position_head = Self::build_position_head(&head_node);
            let volume_node = head_node
                .next_siblings()
                .find(|n| n.has_tag_name("VarVolume"))
                .unwrap();
            let volume = Self::build_instruction_value(&volume_node, VariableType::Float);
            Command::Dispense {
                position_head,
                dispense_all,
                volume,
            }
        } else {
            let volume_node = node
                .descendants()
                .find(|n| n.has_tag_name("Volume"))
                .unwrap();
            let volume = Self::build_instruction_value(&volume_node, VariableType::Float);
            let dispense_all_node = volume_node
                .next_siblings()
                .find(|n| n.has_tag_name("DsAll"))
                .unwrap();
            let dispense_all = Self::build_bool(dispense_all_node.text().unwrap());
            Command::DispenseMainArray {
                volume,
                dispense_all,
            }
        }
    }

    fn build_instruction_eject_tips(node: &Node) -> Command {
        let pos_node = node
            .descendants()
            .find(|n| n.has_tag_name("LoadEjectTipsInstr"))
            .unwrap();
        let l = Self::build_load_eject_tips_head(&pos_node);
        Command::EjectTips {
            load_eject_tips_head: l,
        }
    }

    fn build_instruction_execute_vsta_macro(node: &Node) -> Command {
        let name = node
            .descendants()
            .find(|n| n.has_tag_name("MacroName"))
            .unwrap()
            .text()
            .unwrap()
            .to_string();
        Command::ExecuteVSTAMacro { name }
    }

    fn build_instruction_head_position(node: &Node) -> Command {
        let pos_node = node
            .descendants()
            .find(|n| n.has_tag_name("PositionHeadInstr"))
            .unwrap();
        let position_head = Self::build_position_head(&pos_node);
        Command::HeadPosition { position_head }
    }

    fn build_instruction_home(node: &Node) -> Command {
        let x_node = node.descendants().find(|n| n.has_tag_name("X")).unwrap();
        let y_node = x_node
            .next_siblings()
            .find(|n| n.has_tag_name("Y"))
            .unwrap();
        let z_node = y_node
            .next_siblings()
            .find(|n| n.has_tag_name("Z"))
            .unwrap();
        let x = Self::build_bool(x_node.text().unwrap());
        let y = Self::build_bool(y_node.text().unwrap());
        let z = Self::build_bool(z_node.text().unwrap());
        Command::Home { x, y, z }
    }

    fn build_instruction_if_then(node: &Node) -> Command {
        let if_node = node
            .descendants()
            .find(|n| n.has_tag_name("ControlInstr_IfThen"))
            .unwrap();
        let fields = text_only_children(&if_node);
        let comparator = Self::build_comparator(fields.get(INSTR_COMPARATOR).unwrap());
        let var_type = Self::build_test_variable_type(fields.get(INSTR_TEST_TYPE).unwrap());
        let mut instr_val = Vec::new();
        for c in if_node.children().filter(|n| n.is_element()).skip(2) {
            instr_val.push(Self::build_instruction_value(&c, var_type));
        }
        let rhs = instr_val.pop().unwrap();
        let lhs = instr_val.pop().unwrap();
        Command::IfThen {
            comparator,
            lhs,
            rhs,
        }
    }

    fn build_instruction_load_tips(node: &Node) -> Command {
        let pos_node = node
            .descendants()
            .find(|n| n.has_tag_name("LoadEjectTipsInstr"))
            .unwrap();
        let l = Self::build_load_eject_tips_head(&pos_node);
        Command::LoadTips {
            load_eject_tips_head: l,
        }
    }

    fn build_instruction_math_operation(node: &Node) -> Command {
        let math_node = node
            .descendants()
            .find(|n| n.has_tag_name("ControlInstr_MathOps"))
            .unwrap();
        let instr_type = VariableType::Float;
        let mut operator = None;
        let mut vars = Vec::new();
        for c in math_node.children().filter(|n| n.is_element()) {
            if c.has_tag_name("DataType") {
                continue;
            } else if c.has_tag_name("Operator") {
                operator = Some(Self::build_operator(c.text().unwrap()));
            } else {
                vars.push(Self::build_instruction_value(&c, instr_type));
            }
        }
        let rhs_op2 = vars.pop().unwrap();
        let rhs_op1 = vars.pop().unwrap();
        let lhs = vars.pop().unwrap();
        Command::MathOperation {
            operator: operator.unwrap(),
            lhs,
            rhs_op1,
            rhs_op2,
        }
    }

    fn build_instruction_mix(node: &Node) -> Command {
        let head_node = node
            .descendants()
            .find(|n| n.has_tag_name("PositionHeadInstr"))
            .unwrap();
        let position_head = Self::build_position_head(&head_node);
        Command::Mix { position_head }
    }

    fn build_instruction_move_material(node: &Node) -> Command {
        let from_node = node
            .descendants()
            .find(|n| n.has_tag_name("MoveMatPickInstr"))
            .unwrap();
        let from_head_node = from_node
            .descendants()
            .find(|n| n.has_tag_name("PositionHeadInstr"))
            .unwrap();
        let from = Self::build_position_head(&from_head_node);
        let to_node = from_node
            .next_siblings()
            .find(|n| n.has_tag_name("MoveMatPlaceInstr"))
            .unwrap();
        let to_head_node = to_node
            .descendants()
            .find(|n| n.has_tag_name("PositionHeadInstr"))
            .unwrap();
        let to = Self::build_position_head(&to_head_node);
        Command::MoveMaterial { from, to }
    }

    fn build_instruction_pick(node: &Node) -> Command {
        let pos_node = node
            .descendants()
            .find(|n| n.has_tag_name("HeadPosInstr"))
            .unwrap();
        let position_head = Self::build_position_head(&pos_node);
        Command::Pick { position_head }
    }

    fn build_instruction_place(node: &Node) -> Command {
        let pos_node = node
            .descendants()
            .find(|n| n.has_tag_name("HeadPosInstr"))
            .unwrap();
        let position_head = Self::build_position_head(&pos_node);
        Command::Place { position_head }
    }

    fn build_instruction_run_method(node: &Node) -> Command {
        let call_method_uid = node
            .descendants()
            .find(|n| n.has_tag_name("CalledMethod"))
            .unwrap()
            .text()
            .unwrap();

        let param_node = node
            .descendants()
            .find(|n| n.has_tag_name("Parameters"))
            .unwrap();
        let mut parameters = Vec::new();
        for c in param_node.children().filter(|n| n.is_element()).skip(1) {
            parameters.push(Self::build_parameter(&c));
        }
        Command::RunMethod {
            method: call_method_uid.parse().unwrap(),
            parameters,
        }
    }

    fn build_instruction_run_shaker_for_time(node: &Node) -> Command {
        let speed_node = node
            .descendants()
            .find(|n| n.has_tag_name("Speed"))
            .unwrap();
        let speed = Self::build_instruction_value(&speed_node, VariableType::Float);
        let timeout_node = speed_node
            .next_siblings()
            .find(|n| n.has_tag_name("TimeoutDuration"))
            .unwrap();
        let timeout = Self::build_instruction_value(&timeout_node, VariableType::Seconds);
        Command::RunShakerForTime { speed, timeout }
    }

    fn build_instruction_rem(node: &Node) -> Command {
        let msg_node = node
            .descendants()
            .find(|n| n.has_tag_name("CommentText"))
            .unwrap();
        let comment = match msg_node.text() {
            Some(s) => s.to_string(),
            None => "".to_string(),
        };
        Command::REM { comment }
    }

    fn build_instruction_set_light_intensity(node: &Node) -> Command {
        let light_node = node
            .descendants()
            .find(|n| n.has_tag_name("LegLightPercentage"))
            .unwrap();
        let percentage = Self::build_instruction_value(&light_node, VariableType::Float);
        Command::SetLegLightIntensity { percentage }
    }

    fn build_instruction_set_speed(node: &Node) -> Command {
        let speed_node = node
            .descendants()
            .find(|n| n.has_tag_name("Speed"))
            .unwrap();
        let speed = Self::build_instruction_value(&speed_node, VariableType::Float);
        Command::SetSpeed { speed }
    }

    fn build_instruction_shaker_on_off(node: &Node) -> Command {
        let device = node
            .descendants()
            .find(|n| n.has_tag_name("DCCControl"))
            .unwrap()
            .text()
            .unwrap()
            .to_string();
        let on_off_node = node
            .descendants()
            .find(|n| n.has_tag_name("TurnOn"))
            .unwrap();
        let on_off = Self::build_instruction_value(&on_off_node, VariableType::Bool);
        Command::ShakerOnOff { device, on_off }
    }

    fn build_instruction_while_loop(node: &Node) -> Command {
        let if_node = node
            .descendants()
            .find(|n| n.has_tag_name("ControlInstr_WhileLoop"))
            .unwrap();
        let fields = text_only_children(&if_node);
        let comparator = Self::build_comparator(fields.get(INSTR_COMPARATOR).unwrap());
        let var_type = Self::build_test_variable_type(fields.get("ComparisonType").unwrap());
        let mut instr_val = Vec::new();
        for c in if_node.children().filter(|n| n.is_element()).skip(2) {
            instr_val.push(Self::build_instruction_value(&c, var_type));
        }
        let rhs = instr_val.pop().unwrap();
        let lhs = instr_val.pop().unwrap();
        Command::IfThen {
            comparator,
            lhs,
            rhs,
        }
    }

    fn build_show_dialog(node: &Node) -> Command {
        let msg_node = node
            .descendants()
            .find(|n| n.has_tag_name("DisplayText"))
            .unwrap();
        Command::ShowDialog {
            text: msg_node.text().unwrap().to_string(),
        }
    }

    fn build_instruction_temperature_on_off(node: &Node) -> Command {
        let fields = text_only_children(&node);
        let device = fields.get("DCCControl").unwrap().to_string();
        let temp_node = node
            .descendants()
            .find(|n| n.has_tag_name("TurnOn"))
            .unwrap();
        let on_off = Self::build_instruction_value(&temp_node, VariableType::Bool);
        Command::TemperatureOnOff { device, on_off }
    }

    fn build_instruction_set_temperature(node: &Node) -> Command {
        let device_node = node
            .descendants()
            .find(|n| n.has_tag_name("DCCControl"))
            .unwrap();
        let device = device_node.text().unwrap().to_string();
        let temp_node = node
            .descendants()
            .find(|n| n.has_tag_name("Temperature"))
            .unwrap();
        let temperature = Self::build_instruction_value(&temp_node, VariableType::Float);
        Command::SetTemperature {
            device,
            temperature,
        }
    }

    fn build_instruction_value(node: &Node, value_type: VariableType) -> InstructionValue {
        let fields = text_only_children(node);
        let value_str = fields.get(INSTR_DIRECT_VALUE).unwrap();
        let var_str = fields.get(INSTR_VARIABLE).unwrap();
        let var: Option<Uuid> = if *var_str == "[[[[---NONE---]]]]" {
            None
        } else {
            Some(var_str.parse().unwrap())
        };
        let value = match value_type {
            VariableType::Bool => {
                let b = Self::build_bool(&value_str);
                VariableValue::Bool(b)
            }
            VariableType::Float => VariableValue::Float(value_str.parse().unwrap()),
            VariableType::Int => VariableValue::Int(value_str.parse().unwrap()),
            VariableType::String => VariableValue::String(value_str.to_string()),
            VariableType::Seconds => VariableValue::Seconds(value_str.parse().unwrap()),
        };
        InstructionValue {
            variable: var,
            direct: value,
        }
    }
}

/// The state of the Maestro application when it was saved. The Maestro export format may change, but
/// this class will strive to provide a constant access API.
///
/// # Example
///
/// ```
/// // Read the XML string of an empty application
/// let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
/// d.push("resources/test/Applications_Empty.eap");
/// let empty_app = std::fs::read_to_string(d).unwrap();
///
///let app = maestro_application::Loader::new(&empty_app).build_application();
/// ```
///
pub struct SavedApplication {
    start_method: Uuid,
    global_variables: HashMap<Uuid, Variable>,
    layouts: HashMap<Uuid, Layout>,
    methods: HashMap<Uuid, Method>,
}

impl SavedApplication {
    fn set_global_variables(&mut self, pool: VariablesPool) {
        self.global_variables = pool.variables;
    }

    fn add_layout(&mut self, layout: Layout) {
        self.layouts.insert(layout.id, layout);
    }

    fn add_method(&mut self, method: Method) {
        self.methods.insert(method.id, method);
    }

    /// Global variables of saved application
    pub fn global_variables(&self) -> &HashMap<Uuid, Variable> {
        &self.global_variables
    }

    /// Does method exist
    pub fn has_method(&self, method_id: Uuid) -> bool {
        self.methods.contains_key(&method_id)
    }

    /// The layout ids of the application
    pub fn ids_global_var(&self) -> Vec<&Uuid> {
        self.global_variables.keys().collect()
    }

    /// The layout ids of the application
    pub fn ids_layout(&self) -> Vec<&Uuid> {
        self.layouts.keys().collect()
    }

    /// The method ids of the application
    pub fn ids_methods(&self) -> Vec<&Uuid> {
        self.methods.keys().collect()
    }

    /// Instruction from method
    pub fn instruction(&self, method_id: Uuid, line: usize) -> Option<&Instruction> {
        self.methods.get(&method_id).and_then(|m| m.instructions.get(line))
    }

    /// How many instructions in the method
    pub fn instruction_count(&self, method_id: Uuid) -> Option<usize> {
        self.methods.get(&method_id).and_then(|m| Some(m.instructions.len()))
    }

    /// The layout associated with the specified method
    pub fn layout_of_method(&self, method_id: Uuid) -> Option<Uuid> {
        match self.methods.get(&method_id) {
            Some(method) => Some(method.layout_id),
            None => None,
        }
    }

    /// Local variables of a method
    pub fn local_variables_of_method(&self, method_id: Uuid) -> Option<&HashMap<Uuid, Variable>> {
        self.methods
            .get(&method_id)
            .and_then(|m| Some(&m.local_variables_pool.variables))
    }

    /// The name of the global variable
    pub fn name_global_var(&self, var_id: Uuid) -> Option<&str> {
        match self.global_variables.get(&var_id) {
            Some(var) => Some(&var.designation),
            None => None,
        }
    }

    /// The name of the layout
    pub fn name_layout(&self, layout_id: Uuid) -> Option<&str> {
        match self.layouts.get(&layout_id) {
            Some(pool) => Some(&pool.designation),
            None => None,
        }
    }

    /// The name of the method
    pub fn name_method(&self, method_id: Uuid) -> Option<&str> {
        match self.methods.get(&method_id) {
            Some(method) => Some(&method.designation),
            None => None,
        }
    }

    /// Parameters of a method
    pub fn parameters_of_method(&self, method_id: Uuid) -> Option<&HashMap<Uuid, Variable>> {
        self.methods
            .get(&method_id)
            .and_then(|m| Some(&m.parameters.variables))
    }

    /// The method that called at the start of the application
    pub fn start_method(&self) -> Uuid {
        self.start_method
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum VariableValue {
    Bool(bool),
    Float(f64),
    Int(u32),
    String(String),
    Seconds(u32),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum VariableType {
    Bool,
    Float,
    Int,
    String,
    Seconds,
}

struct VariablesPool {
    designation: String,
    id: Uuid,
    variables: HashMap<Uuid, Variable>,
}
#[derive(Debug, Clone)]
pub struct Variable {
    designation: String,
    id: Uuid,
    value: VariableValue,
}

struct Layout {
    designation: String,
    id: Uuid,
    positions: HashMap<Uuid, Location>,
}

struct Location {
    id: Uuid,
    position: String,
    number_stacked: u32,
    designation: String,
    consumable: Uuid,
}

struct Method {
    designation: String,
    id: Uuid,
    layout_id: Uuid,
    local_variables_pool: VariablesPool,
    parameters: VariablesPool,
    instructions: Vec<Instruction>,
}

pub struct Instruction {
    pub is_comment: bool,
    pub command: Command,
}

#[derive(Debug)]
pub enum Command {
    AbsoluteMove,
    ApplicationExit,
    Aspirate {
        position_head: PositionHead,
        volume: InstructionValue,
    },
    BeginLoop {
        index: InstructionValue,
        from: InstructionValue,
        to: InstructionValue,
        steps: InstructionValue,
    },
    CloseWorkbook,
    Dispense {
        position_head: PositionHead,
        volume: InstructionValue,
        dispense_all: bool,
    },
    DispenseMainArray {
        volume: InstructionValue,
        dispense_all: bool,
    },
    EjectTips {
        load_eject_tips_head: LoadEjectTipsHead,
    },
    EndIf,
    EndLoop,
    EndWhile,
    ExecuteVSTAMacro {
        name: String,
    },
    GetCurrentPositionRelativeToReference,
    HeadPosition {
        position_head: PositionHead,
    },
    Home {
        x: bool,
        y: bool,
        z: bool,
    },
    HomePAxis,
    IfThen {
        comparator: Comparator,
        lhs: InstructionValue,
        rhs: InstructionValue,
    },
    Initialize,
    InitializeSystem,
    LoadTips {
        load_eject_tips_head: LoadEjectTipsHead,
    },
    MathOperation {
        operator: Operator,
        lhs: InstructionValue,
        rhs_op1: InstructionValue,
        rhs_op2: InstructionValue,
    },
    Mix {
        position_head: PositionHead,
    },
    MoveMaterial {
        from: PositionHead,
        to: PositionHead,
    },
    OpenWorkbook,
    PAxisSetPosition,
    Pick {
        position_head: PositionHead,
    },
    Place {
        position_head: PositionHead,
    },
    REM {
        comment: String,
    },
    RelativeMove,
    RunMethod {
        method: Uuid,
        parameters: Vec<Parameter>,
    },
    RunMacro,
    RunShakerForTime {
        speed: InstructionValue,
        timeout: InstructionValue,
    },
    SetLegLightIntensity {
        percentage: InstructionValue,
    },
    SetSpeed {
        speed: InstructionValue,
    },
    SetTemperature {
        device: String,
        temperature: InstructionValue,
    },
    SetTravelHeight,
    SetWorkingDirectory,
    ShakerOnOff {
        device: String,
        on_off: InstructionValue,
    },
    ShowDialog {
        text: String,
    },
    StartTime,
    StopTimer,
    StringOperation,
    TemperatureOnOff {
        device: String,
        on_off: InstructionValue,
    },
    Ungrip,
    VerticalPosition,
    WhileLoop {
        operator: Operator,
        lhs: InstructionValue,
        rhs: InstructionValue,
    },
}

#[derive(Debug)]
pub enum Operator {
    Assign,
    Minus,
    Plus,
}

#[derive(Debug)]
pub enum Comparator {
    Equals,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

#[derive(Debug)]
pub struct InstructionValue {
    direct: VariableValue,
    variable: Option<Uuid>,
}

#[derive(Debug)]
pub struct Parameter {
    id: Uuid,
    value: InstructionValue,
}

#[derive(Debug)]
pub struct PositionHead {
    deck_parameter: Option<Uuid>,
    deck_location: InstructionValue,
    z_offset: InstructionValue,
}

#[derive(Debug)]
pub struct LoadEjectTipsHead {
    deck_parameter: Option<Uuid>,
    deck_location: InstructionValue,
}

fn get_float_text(xml: &Node, tag: &str) -> f64 {
    xml.descendants()
        .find(|n| n.has_tag_name(tag))
        .unwrap()
        .text()
        .unwrap()
        .parse()
        .unwrap()
}

fn get_int_text(xml: &Node, tag: &str) -> u32 {
    xml.descendants()
        .find(|n| n.has_tag_name(tag))
        .unwrap()
        .text()
        .unwrap()
        .parse()
        .unwrap()
}

fn text_only_element<'a, 'b>(node: &Node<'a, 'b>) -> Option<&'a str> {
    if !node.is_element() {
        return None;
    }
    if let Some(text) = node.first_child() {
        if text.is_text() {
            let len = node.children().collect::<Vec<_>>().len();
            if len == 1 {
                text.text()
            } else {
                None
            }
        } else {
            None
        }
    } else {
        // Maestro uses an element with nothing in it <a></a> as ""
        Some("")
    }
}

fn text_only_children<'a, 'b>(node: &Node<'a, 'b>) -> HashMap<&'a str, &'a str> {
    let mut result = HashMap::new();
    for n in node.children() {
        if let Some(text) = text_only_element(&n) {
            result.insert(n.tag_name().name(), text);
        } else {
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use roxmltree::Document;

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
    fn build_empty_application() {
        let doc = load_empty_app();
        let app = Loader::new(&doc).build_application();
        assert_eq!(
            app.start_method(),
            "3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap()
        );
        assert_eq!(
            app.name_layout("BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".parse().unwrap()),
            Some("MainLayout")
        );
        assert_eq!(
            app.name_method("3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap()),
            Some("Main")
        );
        assert_eq!(
            app.layout_of_method("3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap()),
            Some("BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".parse().unwrap())
        );
    }

    #[test]
    fn build_complex_application() {
        let doc = load_complex_app();
        let app = Loader::new(&doc).build_application();
        assert_eq!(app.ids_layout().len(), 11);
        assert_eq!(app.ids_methods().len(), 30);

        // TODO: Lists all available instructions. Not part of test, remove after development
        let mut v = Vec::new();
        let loaded = Document::parse(&doc).unwrap();
        for d in loaded
            .descendants()
            .filter(|n| n.has_tag_name("InstructionDesignation"))
        {
            v.push(d.text())
        }
        v.sort();
        v.dedup();
    }

    #[test]
    fn int_float_parsing() {
        const DATA: &'static str = r#"<ExportedApplication>

  <ExportedApplicationVersion>6.8</ExportedApplicationVersion>

    <ExportedApplicationBuild>6</ExportedApplicationBuild>

</ExportedApplication>"#;
        let doc = Document::parse(DATA).unwrap();
        let version = get_float_text(&doc.root(), "ExportedApplicationVersion");
        let build = get_int_text(&doc.root(), "ExportedApplicationBuild");
        assert_eq!(version, 6.8);
        assert_eq!(build, 6);
    }

    #[test]
    fn single_text_element() {
        const DATA: &'static str = r#"<a>Hello<b>World</b></a>"#;
        let doc = Document::parse(DATA).unwrap();
        assert!(text_only_element(&doc.root().first_child().unwrap()).is_none());
        let text_node = doc.descendants().find(|n| n.has_tag_name("b")).unwrap();
        assert_eq!(text_only_element(&text_node), Some("World"));
    }

    #[test]
    fn test_text_only_children() {
        const DATA: &'static str = r#"<a>A
        <b>B</b>
        <c>
            C
            <d>D</d>
        </c>
        <e> </e>
        <f></f>
        </a>"#;
        let doc = Document::parse(DATA).unwrap();
        let mut result = HashMap::new();
        result.insert("b", "B");
        result.insert("e", " ");
        result.insert("f", "");
        assert_eq!(
            text_only_children(&doc.root().first_child().unwrap()),
            result
        )
    }

    #[test]
    fn variable_pool_parsing() {
        const DATA: &'static str = r#"<VariablesPool>

          <VariablesPoolDesignation>MainLayout</VariablesPoolDesignation>

          <VariablesPoolID>BB37AAC5-102D-4367-B1BA-98B7D1E47EF0</VariablesPoolID>

          <VariablesCount>1</VariablesCount>

          <Variable1>

            <VariableType>2</VariableType>

            <VariableID>12A4FC48-6802-491A-ACE5-871B53197F12</VariableID>

            <VariableDesignation>g_NumberOfTipBoxPerDeck</VariableDesignation>

            <Value>1</Value>

            <VariableDescription>The number of Tip Box per reserve deck. Current NGS configuration supports only one</VariableDescription>

            <PermissibleValues>0-10</PermissibleValues>

            <VariablePoolID>D2EEDFC1-22D6-40FF-8A5D-F81B0960238D</VariablePoolID>

            <VariablePoolDesignation>GLOBAL Variables</VariablePoolDesignation>

          </Variable1>

        </VariablesPool>
        "#;
        let doc = Document::parse(DATA).unwrap();
        let node = doc.root().first_element_child().unwrap();
        let var = Loader::build_variables_pool(&node);
        assert_eq!(
            var.id,
            "BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".parse().unwrap()
        );
        assert_eq!(var.designation, "MainLayout".to_string());
        assert_eq!(var.variables.len(), 1);
    }

    #[test]
    fn method_parsing() {
        let xml_str = load_empty_app();
        let doc = Document::parse(&xml_str).unwrap();
        let method_node = doc
            .descendants()
            .find(|n| n.has_tag_name("Method1"))
            .unwrap();
        let var = Loader::build_method(&method_node);
        assert_eq!(var.designation, "Main".to_string());
        assert_eq!(
            var.id,
            "3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap()
        );
        assert_eq!(
            var.layout_id,
            "BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".parse().unwrap()
        );
        assert_eq!(
            var.local_variables_pool.id,
            "9DC99ADE-3702-4D6A-A34C-489E64D46183".parse().unwrap()
        );
        assert_eq!(
            var.parameters.id,
            "68A3020C-9427-4E0E-9235-F8A40FF66969".parse().unwrap()
        );
    }

    #[test]
    fn variable_parsing() {
        const DATA: &'static str = r#"<Variable2>

          <VariableType>2</VariableType>

          <VariableID>82ADDA04-FE60-4F14-B0C6-81AF2B5E524B</VariableID>

          <VariableDesignation>g_ReservedTipBoxZOffset</VariableDesignation>

          <Value>-10</Value>

          <VariableDescription></VariableDescription>

          <PermissibleValues>-9999999-9999999</PermissibleValues>

          <VariablePoolID>D2EEDFC1-22D6-40FF-8A5D-F81B0960238D</VariablePoolID>

          <VariablePoolDesignation>GLOBAL Variables</VariablePoolDesignation>

        </Variable2>"#;
        let doc = Document::parse(DATA).unwrap();
        let node = doc.root().first_element_child().unwrap();
        let var = Loader::build_variable(&node);
        assert_eq!(var.designation, "g_ReservedTipBoxZOffset".to_string());
        assert_eq!(
            var.id,
            "82ADDA04-FE60-4F14-B0C6-81AF2B5E524B".parse().unwrap()
        );
        assert_eq!(var.value, VariableValue::Float(-10.0));
    }

    #[test]
    fn layout_parsing() {
        const DATA: &'static str = r#"<VariablesPool>

        <VariablesPoolDesignation>MainLayout</VariablesPoolDesignation>

        <VariablesPoolID>1B8A66AB-2BA3-4FDF-8982-A5D364ED9874</VariablesPoolID>

        <VariablesCount>17</VariablesCount>

        <Variable1>

            <VariableType>5</VariableType>

            <VarVersion>Sciclone_4</VarVersion>

            <VariableID>504C5661-C3EB-4CA2-9E7A-A974828D4C68</VariableID>

            <VariableDesignation>D1</VariableDesignation>

            <VariableDescription></VariableDescription>

            <NumberOfStackedConsumables>1</NumberOfStackedConsumables>

            <LocDesignation>D1</LocDesignation>

            <LocInstrument></LocInstrument>

            <MatVersion>Sciclone_4</MatVersion>

            <ThisDesignation>Reserve Tip Box 4(1)</ThisDesignation>

            <ThisIDLocMaterial>F3D8533C-00D4-430C-9C8D-45209A8DFC36</ThisIDLocMaterial>

            <IDAccOrCon>5917e9be-ef73-403a-baeb-ff779944598e</IDAccOrCon>

            <AccOrConType>0</AccOrConType>

            <InitialVolume>1</InitialVolume>

            <UseLLT>False</UseLLT>

        </Variable1>

        </VariablesPool>"#;
        let doc = Document::parse(DATA).unwrap();
        let node = doc.root().first_element_child().unwrap();
        let var = Loader::build_layout(&node);
        assert_eq!(var.designation, "MainLayout".to_string());
        assert_eq!(
            var.id,
            "1B8A66AB-2BA3-4FDF-8982-A5D364ED9874".parse().unwrap()
        );
        let loc = var
            .positions
            .get(&"504C5661-C3EB-4CA2-9E7A-A974828D4C68".parse().unwrap())
            .unwrap();
        assert_eq!(loc.position, "D1".to_string());
    }

    #[test]
    fn instruction_value_parsing() {
        const DATA: &'static str = r#"<ZPosOffset>

        <_DirectValue>0</_DirectValue>

        <_Variable>[[[[---NONE---]]]]</_Variable>

    </ZPosOffset>"#;
        let doc = Document::parse(DATA).unwrap();
        let node = doc.root().first_element_child().unwrap();
        let r = Loader::build_instruction_value(&node, VariableType::Float);
        assert_eq!(r.direct, VariableValue::Float(0.0));
        assert_eq!(r.variable, None);
    }

    #[test]
    fn parameter_parsing() {
        const DATA: &'static str = r#"<Parameter1>

        <ForParameter>4C09727C-1AF0-45D5-B756-BD21A058A7A7</ForParameter>

        <ParameterType>2</ParameterType>

        <_DirectValue>25</_DirectValue>

        <_Variable>[[[[---NONE---]]]]</_Variable>

    </Parameter1>"#;
        let doc = Document::parse(DATA).unwrap();
        let node = doc.root().first_element_child().unwrap();
        let p = Loader::build_parameter(&node);
        assert_eq!(
            p.id,
            "4C09727C-1AF0-45D5-B756-BD21A058A7A7".parse().unwrap()
        );
        assert_eq!(p.value.direct, VariableValue::Float(25.0));
        assert_eq!(p.value.variable, None);
    }
}
