use serde::Serialize;
use std::fs::File;
use std::io::{Result, Write};
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ToolTipContents {
    pub title: String,
    pub r#type: String,
    pub parent: Option<String>,
    pub parent_chain: Option<Vec<String>>,
    pub description: String,
    pub alias: Option<String>,
}

#[allow(dead_code)]
pub fn write_tokens_list_ts<P: AsRef<Path>>(
    file_path: P,
    data: &[(String, ToolTipContents)],
) -> Result<()> {
    let mut file = File::create(file_path)?;

    writeln!(file, "export enum ComponentType {{")?;
    writeln!(file, "  PROGRAM,")?;
    writeln!(file, "  COMMAND,")?;
    writeln!(file, "  SUBCOMMAND,")?;
    writeln!(file, "  FLAG_SHORT,")?;
    writeln!(file, "  FLAG_LONG,")?;
    writeln!(file, "  ARGUMENT,")?;
    writeln!(file, "  OTHER")?;
    writeln!(file, "}}\n")?;

    writeln!(
        file,
        "export const tokensList: Record<string, ToolTipContents> = {{"
    )?;

    for (key, tooltip) in data {
        let title = &tooltip.title;
        let parent = match &tooltip.parent {
            Some(p) => format!("\"{}\"", p),
            None => "null".to_string(),
        };
        let parent_chain = match &tooltip.parent_chain {
            Some(chain) => format!(
                "[{}]",
                chain
                    .iter()
                    .map(|s| format!("\"{}\"", s))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            None => "null".to_string(),
        };
        let alias = match &tooltip.alias {
            Some(a) => format!("\"{}\"", a),
            None => "null".to_string(),
        };

        let escaped_description = tooltip
            .description
            .replace('\n', "\\n")
            .replace('"', "\\\"");

        writeln!(file, "  \"{}\": {{", key)?;
        writeln!(file, "    title: \"{}\",", title)?;
        writeln!(file, "    type: ComponentType.{},", tooltip.r#type)?;
        writeln!(file, "    parent: {},", parent)?;
        writeln!(file, "    parent_chain: {},", parent_chain)?;
        writeln!(file, "    description: \"{}\",", escaped_description)?;
        writeln!(file, "    alias: {}", alias)?;
        writeln!(file, "  }},")?;
    }

    writeln!(file, "}};\n")?;

    writeln!(file, "export type ToolTipContents = {{")?;
    writeln!(file, "  title: string;")?;
    writeln!(file, "  type: ComponentType;")?;
    writeln!(file, "  parent: string | null;")?;
    writeln!(file, "  parent_chain: string[] | null;")?;
    writeln!(file, "  description: string;")?;
    writeln!(file, "  alias: string | null;")?;
    writeln!(file, "}};")?;

    Ok(())
}
