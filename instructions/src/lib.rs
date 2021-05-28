use roxmltree::{Descendants};

fn get_float_text(xml: &mut Descendants, tag: &str) -> f64 {
    xml.find(|n| n.has_tag_name(tag))
        .unwrap()
        .text()
        .unwrap()
        .parse()
        .unwrap()
}

fn get_int_text(xml: &mut Descendants, tag: &str) -> usize {
    xml.find(|n| n.has_tag_name(tag))
        .unwrap()
        .text()
        .unwrap()
        .parse()
        .unwrap()
}

fn get_string_text(xml: &mut Descendants, tag: &str) -> String {
    xml.find(|n| n.has_tag_name(tag))
        .unwrap()
        .text()
        .unwrap()
        .parse()
        .unwrap()
}

#[derive(Debug, Eq, PartialEq)]
struct VariablesPool {
    designation: String,
    id: String,
}

impl VariablesPool {
    fn from_xml(xml: &mut Descendants) -> Self {
        VariablesPool {
            designation: get_string_text(xml, "VariablesPoolDesignation"),
            id: get_string_text(xml, "VariablesPoolID"),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Method {
    designation: String,
    program_id: String,
    layout_id: String,
    local_variables_pool: VariablesPool,
    parameters: VariablesPool,
}

impl Method {
    fn from_xml(xml: &mut Descendants) -> Self {
        Method {
            designation: get_string_text(xml, "MethodDesignation"),
            program_id: get_string_text(xml, "ProgramID"),
            layout_id: get_string_text(xml, "LayoutID"),
            local_variables_pool: VariablesPool::from_xml(xml),
            parameters: VariablesPool::from_xml(xml),
        }
    }
}

#[cfg(test)]
mod tests {
    use roxmltree::{Document};
    use super::*;
    #[test]
    fn text_parsing() {
        const DATA: &'static str = r#"<ExportedApplication>

  <ExportedApplicationVersion>6.8</ExportedApplicationVersion>

    <ExportedApplicationBuild>6</ExportedApplicationBuild>

</ExportedApplication>"#;
        let doc = Document::parse(DATA).unwrap();
        let mut nodes = doc.descendants();
        let version = get_float_text(&mut nodes, "ExportedApplicationVersion");
        let build = get_int_text(&mut nodes, "ExportedApplicationBuild");
        assert_eq!(version, 6.8);
        assert_eq!(build, 6);
    }

    #[test]
    fn variable_pool_parsing() {
        const DATA: &'static str = r#"<VariablesPool>

          <VariablesPoolDesignation>MainLayout</VariablesPoolDesignation>

          <VariablesPoolID>BB37AAC5-102D-4367-B1BA-98B7D1E47EF0</VariablesPoolID>

          <VariablesCount>0</VariablesCount>

        </VariablesPool>
        "#;
        let doc = Document::parse(DATA).unwrap();
        let mut nodes = doc.descendants();
        let var = VariablesPool::from_xml(&mut nodes);
        assert_eq!(
            var,
            VariablesPool{
                designation: "MainLayout".to_string(),
                id: "BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".to_string(),
            }
        )
    }

    #[test]
    fn method_parsing() {
        const DATA: &'static str = r#"<Method1>

        <MethodDesignation>Main</MethodDesignation>

        <ProgramID>3AC47C04-DCCE-4036-8F9F-6AD7D530E220</ProgramID>

        <LayoutID>BB37AAC5-102D-4367-B1BA-98B7D1E47EF0</LayoutID>

        <LocalVariablesPool>

          <VariablesPool>

            <VariablesPoolDesignation>Main:LOCAL Variables</VariablesPoolDesignation>

            <VariablesPoolID>9DC99ADE-3702-4D6A-A34C-489E64D46183</VariablesPoolID>

            <VariablesCount>0</VariablesCount>

          </VariablesPool>

        </LocalVariablesPool>

        <Parameters>

          <VariablesPool>

            <VariablesPoolDesignation>Main:Parameters</VariablesPoolDesignation>

            <VariablesPoolID>68A3020C-9427-4E0E-9235-F8A40FF66969</VariablesPoolID>

            <VariablesCount>0</VariablesCount>

          </VariablesPool>

        </Parameters>

        <Hidden>0</Hidden>

        <ReadOnly>0</ReadOnly>

        <MethodDescription></MethodDescription>

        <MethodVisibleToClient>-1</MethodVisibleToClient>

        <DefaultErrorHandler></DefaultErrorHandler>

        <ProgramExecutionTime>0</ProgramExecutionTime>

        <ProgramCustomProperty></ProgramCustomProperty>

        <HideParametersDialog>0</HideParametersDialog>

        <InstructionsCount>0</InstructionsCount>

      </Method1>"#;
        let doc = Document::parse(DATA).unwrap();
        let mut nodes = doc.descendants();
        let var = Method::from_xml(&mut nodes);
        assert_eq!(var.designation, "Main".to_string());
        assert_eq!(var.program_id, "3AC47C04-DCCE-4036-8F9F-6AD7D530E220".to_string());
        assert_eq!(var.layout_id, "BB37AAC5-102D-4367-B1BA-98B7D1E47EF0".to_string());
        assert_eq!(var.local_variables_pool.id, "9DC99ADE-3702-4D6A-A34C-489E64D46183".to_string());
        assert_eq!(var.parameters.id, "68A3020C-9427-4E0E-9235-F8A40FF66969".to_string());
    }
}
