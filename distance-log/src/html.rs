use chrono::Utc;
use failure::{Error, ResultExt};
use handlebars::Handlebars;
use serde_derive::{Deserialize, Serialize};
use std::{fs::File, path::Path};

#[derive(Debug, Serialize, Deserialize)]
pub struct TableEntry {
    pub map_name: String,
    pub map_author: Option<String>,
    pub map_preview: Option<String>,
    pub mode: String,
    pub new_recordholder: String,
    pub old_recordholder: Option<String>,
    pub record_new: String,
    pub record_old: Option<String>,
    pub workshop_item_id: Option<String>,
    pub steam_id_author: Option<String>,
    pub steam_id_new_recordholder: String,
    pub steam_id_old_recordholder: Option<String>,
    pub fetch_time: String,
}

#[derive(Debug, Serialize)]
struct PageData {
    entries: Box<[TableEntry]>,
    update_time: String,
}

pub fn render(
    template: impl AsRef<Path>,
    entries: Box<[TableEntry]>,
    output: impl AsRef<Path>,
) -> Result<(), Error> {
    let page_data = PageData {
        entries,
        update_time: Utc::now().to_rfc2822(),
    };

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars
        .render_template_source_to_write(
            &mut File::open(template)?,
            &page_data,
            &mut File::create(output)?,
        )
        .context("error writing rendered template to file")?;

    Ok(())
}
