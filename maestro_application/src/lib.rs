use roxmltree::{Descendants, Document, Node};
use std::collections::HashMap;
use std::hash::Hash;
use uuid::Uuid;

const APP: &str = "Application";
const APP_BUILD: &str = "ExportedApplicationBuild";
const APP_VERSION: &str = "ExportedApplicationVersion";
const GLOBAL_VAR_POOL: &str = "GlobalVariablesPool";
const LAYOUT_ID: &str = "LayoutID";
const LAYOUTS: &str = "Layouts";
const LAYOUTS_COUNT: &str = "LayoutsCount";
const LOCAL_VAR_POOL: &str = "LocalVariablesPool";
const METHODS: &str = "Methods";
const METHODS_COUNT: &str = "MethodsCount";
const METHOD_DESIG: &str = "MethodDesignation";
const PARAMS: &str = "Parameters";
const PROGRAM_ID: &str = "ProgramID";
const START_METHOD: &str = "StartupMethod";
const VAR_COUNT: &str = "VariablesCount";
const VAR_DESIG: &str = "VariableDesignation";
const VAR_ID: &str = "VariableID";
const VAR_POOL_DESIG: &str = "VariablesPoolDesignation";
const VAR_POOL_ID: &str = "VariablesPoolID";
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
                    let layout_var =
                        Self::build_variables_pool(&layouts.first_element_child().unwrap());
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
            "2" => Some(VariableValue::Int(val_str.parse().unwrap())),
            "3" => Some(VariableValue::String(val_str.to_string())),
            "4" => {
                let b = if *val_str == "0" {false} else {true};
                Some(VariableValue::Bool(b))
            },

            _ => None
        };
        Variable {
            designation: variable_fields.get(VAR_DESIG).unwrap().to_string(),
            id: variable_fields.get(VAR_ID).unwrap().parse().unwrap(),
            value: value.unwrap(),
        }
    }

    fn build_variables_pool(node: &Node) -> VariablesPool {
        let global_fields = text_only_children(node);
        let var_count = node.descendants().find(|n| n.has_tag_name(VAR_COUNT)).unwrap();
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

    fn build_method(node: &Node) -> Method {
        let method_fields = text_only_children(node);
        let mut local_var: Option<VariablesPool> = None;
        let mut params: Option<VariablesPool> = None;
        for c in node.children() {
            if c.has_tag_name(LOCAL_VAR_POOL) {
                local_var = Some(Self::build_variables_pool(
                    &c.first_element_child().unwrap(),
                ));
            } else if c.has_tag_name(PARAMS) {
                params = Some(Self::build_variables_pool(
                    &c.first_element_child().unwrap(),
                ));
            }
        }
        Method {
            designation: method_fields.get(METHOD_DESIG).unwrap().parse().unwrap(),
            id: method_fields.get(PROGRAM_ID).unwrap().parse().unwrap(),
            layout_id: method_fields.get(LAYOUT_ID).unwrap().parse().unwrap(),
            local_variables_pool: local_var.unwrap(),
            parameters: params.unwrap(),
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
    layouts: HashMap<Uuid, VariablesPool>,
    methods: HashMap<Uuid, Method>,
}

impl SavedApplication {
    fn set_global_variables(&mut self, pool: VariablesPool) {
        self.global_variables = pool.variables;
    }

    fn add_layout(&mut self, pool: VariablesPool) {
        self.layouts.insert(pool.id, pool);
    }

    fn add_method(&mut self, method: Method) {
        self.methods.insert(method.id, method);
    }

    /// The layout ids of the application
    pub fn ids_layout(&self) -> Vec<&Uuid> {
        self.layouts.keys().collect()
    }

    /// The method ids of the application
    pub fn ids_methods(&self) -> Vec<&Uuid> {
        self.methods.keys().collect()
    }

    /// The layout associated with the specified method
    pub fn layout_of_method(&self, method_id:Uuid) -> Option<Uuid> {
        match self.methods.get(&method_id) {
            Some(method) => Some(method.layout_id),
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

    /// The method that called at the start of the application
    pub fn start_method(&self) -> Uuid {
        self.start_method
    }
}

#[derive(Debug, Eq, PartialEq)]
enum VariableValue {
    Bool(bool),
    Int(i32),
    String(String),
}

#[derive(Debug, Eq, PartialEq)]
struct VariablesPool {
    designation: String,
    id: Uuid,
    variables: HashMap<Uuid, Variable>,
}
#[derive(Debug, Eq, PartialEq)]
struct Variable {
    designation: String,
    id: Uuid,
    value: VariableValue
}

#[derive(Debug, Eq, PartialEq)]
struct Method {
    designation: String,
    id: Uuid,
    layout_id: Uuid,
    local_variables_pool: VariablesPool,
    parameters: VariablesPool,
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

fn get_string_text<'a, 'b>(xml: &'a Node, tag: &'b str) -> &'a str {
    xml.descendants()
        .find(|n| n.has_tag_name(tag))
        .unwrap()
        .text()
        .unwrap()
}

fn text_only_element<'a, 'b>(node: &Node<'a, 'b>) -> Option<&'a str> {
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
        None
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
        d.push("resources/test/Applications_Empty.eap");

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
        </a>"#;
        let doc = Document::parse(DATA).unwrap();
        let mut result = HashMap::new();
        result.insert("b", "B");
        assert_eq!(
            text_only_children(&doc.root().first_child().unwrap()),
            result
        )
    }

    #[test]
    fn str_parsing() {
        const DATA: &'static str = r#"<ExportedApplication>

        <StartupMethod>3AC47C04-DCCE-4036-8F9F-6AD7D530E220</StartupMethod>

        </ExportedApplication>"#;
        let doc = Document::parse(DATA).unwrap();
        let node = doc.root();
        let text = get_string_text(&node, "StartupMethod");
        assert_eq!(text, "3AC47C04-DCCE-4036-8F9F-6AD7D530E220");
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
        assert_eq!(var.id, "BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".parse().unwrap());
        assert_eq!(var.designation, "MainLayout".to_string());
        assert_eq!(var.variables.len(), 1);
    }

    #[test]
    fn method_parsing() {
        let xml_str = load_empty_app();
        let doc = Document::parse(&xml_str).unwrap();
        let method_node = doc.descendants().find(|n| n.has_tag_name("Method1")).unwrap();
        let var = Loader::build_method(&method_node);
        assert_eq!(var.designation, "Main".to_string());
        assert_eq!(var.id, "3AC47C04-DCCE-4036-8F9F-6AD7D530E220".parse().unwrap());
        assert_eq!(var.layout_id, "BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".parse().unwrap());
        assert_eq!(var.local_variables_pool.id, "9DC99ADE-3702-4D6A-A34C-489E64D46183".parse().unwrap());
        assert_eq!(var.parameters.id, "68A3020C-9427-4E0E-9235-F8A40FF66969".parse().unwrap());
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
        assert_eq!(var.id, "82ADDA04-FE60-4F14-B0C6-81AF2B5E524B".parse().unwrap());
        assert_eq!(var.value, VariableValue::Int(-10));
    }
}
