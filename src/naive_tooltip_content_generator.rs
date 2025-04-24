use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct ToolTipContents {
    title: Option<String>,
    #[serde(rename = "type")]
    component_type: String, // Will be mapped to enum variant
    parent: Option<String>,
    parent_chain: Option<Vec<String>>,
    description: String,
    examples: Option<Vec<serde_json::Value>>, // You can replace with concrete type
    references: Option<Vec<serde_json::Value>>, // You can replace with concrete type
    alias: Option<String>,
}

type TokenObject = HashMap<String, ToolTipContents>;

const TYPE_DEFS: &str = r#"
enum ToolTipExampleMediaType {
  Image = "image",
  Video = "video",
  Gif = "gif",
}

enum ReferenceType {
  Guide = "guide",
  DocsReference = "docsReference",
  Tutorial = "tutorial",
  Video = "video",
  BlogPost = "blogPost",
  External = "external",
  Example = "example",
}

type Reference = {
  name?: string;
  description?: string;
  r#type: ReferenceType;
  url: string;
}

type ToolTipExampleMedia = {
  r#type: ToolTipExampleMediaType;
  src: string;
}

type ToolTipContentsExample = {
  code?: string;
  description?: string;
  title?: string;
  media?: ToolTipExampleMedia[]
}

enum ComponentType {
  PROGRAM,
  COMMAND,
  SUBCOMMAND,
  FLAG_SHORT,
  FLAG_LONG,
  ARGUMENT,
  OTHER
}

type ToolTipContents = {
  title: Option<string>;
  r#type: ComponentType;
  parent: string|null;
  parent_chain: string[]|null;
  description: string;
  examples?: ToolTipContentsExample[];
  references?: Reference[];
  alias: string|null;
}

type TokenObject = {
  [key: string]: ToolTipContents
}
"#;

fn serialize_token_object_to_ts(token_map: &TokenObject) -> String {
    let mut out = String::from("const tokensList: TokenObject = {\n");

    for (key, val) in token_map {
      
        let type_str = format!("ComponentType.{}", val.r#component_type.to_uppercase());

        // let type_str = format!("ComponentType.{}", val.component_type.to_uppercase());

        let parent = val.parent
            .as_ref()
            .map(|p| format!("\"{}\"", p))
            .unwrap_or("null".to_string());

        let chain = val.parent_chain
            .as_ref()
            .map(|c| format!("[{}]", c.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")))
            .unwrap_or("null".to_string());

        let alias = val.alias
            .as_ref()
            .map(|a| format!("\"{}\"", a))
            .unwrap_or("null".to_string());

        let title = val.title.as_ref().map(String::as_str).unwrap_or("<missing title>");

        out.push_str(&format!(
            "  \"{}\": {{\n    title: \"{}\",\n    type: {},\n    parent: {},\n    parent_chain: {},\n    description: \"{}\",\n    alias: {}\n  }},\n",
            key, title.replace('"', "\\\""), type_str, parent, chain, val.description.replace('"', "\\\""), alias
        ));
    }

    out.push_str("};\n");
    out
}

pub fn write_ts_file(cli_json_path: &PathBuf, output_ts_path: &PathBuf) -> std::io::Result<()> {
    let cli_file = File::open(cli_json_path).expect("Failed to open CLI structure JSON file");
    let token_data: TokenObject = serde_json::from_reader(cli_file).expect("Failed to parse CLI structure JSON");

    let mut file = File::create(output_ts_path).expect("Failed to create TypeScript output file");
    file.write_all(TYPE_DEFS.as_bytes()).expect("Failed to write type definitions");
    file.write_all(b"\n\n").expect("Failed to write spacing");

    let ts_data = serialize_token_object_to_ts(&token_data);
    file.write_all(ts_data.as_bytes()).expect("Failed to write serialized token object");

    Ok(())
}
