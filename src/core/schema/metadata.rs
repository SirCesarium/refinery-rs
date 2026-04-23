use crate::core::schema::artifacts::Binary;
use crate::errors::{RefineryError, Result};
use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

/// Metadata extracted from `Cargo.toml`
#[derive(Debug, Default, Clone)]
pub struct CargoMetadata {
    pub name: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub repository: String,
}

/// Updates `Cargo.toml` content with required metadata for installers.
///
/// # Errors
/// Returns error if TOML parsing fails or required structures cannot be created.
pub fn update_cargo_toml_with_metadata(content: &str) -> Result<String> {
    let mut cargo_toml = content.parse::<DocumentMut>()?;
    let metadata = get_cargo_metadata(&cargo_toml);

    ensure_table_exists(&mut cargo_toml, "package")?;
    let package = cargo_toml["package"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("package is not a table".into()))?;

    ensure_sub_table_exists(package, "metadata")?;
    let metadata_table = package["metadata"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("metadata is not a table".into()))?;

    ensure_sub_table_exists(metadata_table, "deb")?;
    metadata_table["deb"]["name"] = value(&metadata.name);

    ensure_sub_table_exists(metadata_table, "generate-rpm")?;
    let rpm = metadata_table["generate-rpm"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("generate-rpm is not a table".into()))?;
    rpm["name"] = value(&metadata.name);

    if rpm.get("assets").is_none() {
        let mut assets = Array::new();
        let mut asset = toml_edit::InlineTable::new();
        asset.insert(
            "source",
            Value::from(format!("target/release/{}", metadata.name)),
        );
        asset.insert("dest", Value::from(format!("/usr/bin/{}", metadata.name)));
        asset.insert("mode", Value::from("755"));
        assets.push(asset);
        rpm["assets"] = value(assets);
    }

    Ok(cargo_toml.to_string())
}

/// Injects specific fields into the `[package]` section of `Cargo.toml`.
///
/// # Errors
/// Returns error if TOML parsing fails.
pub fn inject_cargo_fields(
    content: &str,
    authors: Option<Vec<String>>,
    license: Option<String>,
    description: Option<String>,
    repository: Option<String>,
) -> Result<String> {
    let mut cargo_toml = content.parse::<DocumentMut>()?;
    ensure_table_exists(&mut cargo_toml, "package")?;
    let pkg = cargo_toml["package"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("package is not a table".into()))?;

    if let Some(a) = authors {
        let mut array = Array::new();
        for author in a {
            array.push(author);
        }
        pkg["authors"] = value(array);
    }
    if let Some(l) = license {
        pkg["license"] = value(l);
    }
    if let Some(d) = description {
        pkg["description"] = value(d);
    }
    if let Some(r) = repository {
        pkg["repository"] = value(r);
    }

    Ok(cargo_toml.to_string())
}

/// Prepares `Cargo.toml` with binary definitions.
///
/// # Errors
/// Returns error if TOML parsing fails.
pub fn prepare_cargo_bins(content: &str, bins: &[Binary]) -> Result<String> {
    let mut cargo_toml = content.parse::<DocumentMut>()?;

    // Remove existing bin sections
    cargo_toml.remove("bin");

    if bins.is_empty() {
        return Ok(cargo_toml.to_string());
    }

    let mut bin_array = toml_edit::ArrayOfTables::new();
    for bin in bins {
        let mut table = Table::new();
        table.insert("name", value(bin.name.clone()));
        table.insert("path", value(bin.path.clone()));
        bin_array.push(table);
    }

    cargo_toml.insert("bin", Item::ArrayOfTables(bin_array));

    Ok(cargo_toml.to_string())
}

/// Prepares `Cargo.toml` for library export and optionally `cbindgen`.
///
/// # Errors
/// Returns error if TOML parsing fails.
pub fn prepare_cargo_lib(
    content: &str,
    name: &str,
    crate_types: Vec<String>,
    cbindgen: bool,
) -> Result<String> {
    let mut cargo_toml = content.parse::<DocumentMut>()?;
    let metadata = get_cargo_metadata(&cargo_toml);

    ensure_table_exists(&mut cargo_toml, "lib")?;
    let lib = cargo_toml["lib"]
        .as_table_mut()
        .ok_or_else(|| RefineryError::Config("lib is not a table".into()))?;

    let mut final_name = name.replace('-', "_");
    if final_name == metadata.name.replace('-', "_") {
        final_name.push_str("_lib");
    }
    lib["name"] = value(final_name);

    let mut types = Array::new();
    for t in crate_types {
        types.push(t);
    }
    lib["crate-type"] = value(types);

    if cbindgen {
        ensure_table_exists(&mut cargo_toml, "build-dependencies")?;
        cargo_toml["build-dependencies"]["cbindgen"] = value("0.27");
    }

    Ok(cargo_toml.to_string())
}

#[must_use]
pub fn get_cargo_metadata(doc: &DocumentMut) -> CargoMetadata {
    let pkg = doc.get("package");
    CargoMetadata {
        name: pkg
            .and_then(|p| p.get("name"))
            .and_then(Item::as_str)
            .unwrap_or("project")
            .into(),
        description: pkg
            .and_then(|p| p.get("description"))
            .and_then(Item::as_str)
            .unwrap_or("")
            .into(),
        authors: pkg
            .and_then(|p| p.get("authors"))
            .and_then(Item::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        license: pkg
            .and_then(|p| p.get("license"))
            .and_then(Item::as_str)
            .unwrap_or("")
            .into(),
        repository: pkg
            .and_then(|p| p.get("repository"))
            .and_then(Item::as_str)
            .unwrap_or("")
            .into(),
    }
}

fn ensure_table_exists(doc: &mut DocumentMut, key: &str) -> Result<()> {
    if doc.get(key).is_none() {
        doc.insert(key, Item::Table(Table::new()));
    } else if !doc[key].is_table() {
        return Err(RefineryError::Config(format!(
            "Key '{key}' in Cargo.toml is not a table"
        )));
    }
    Ok(())
}

fn ensure_sub_table_exists(table: &mut Table, key: &str) -> Result<()> {
    if table.get(key).is_none() {
        table.insert(key, Item::Table(Table::new()));
    } else if !table[key].is_table() {
        return Err(RefineryError::Config(format!(
            "Sub-key {key} in Cargo.toml is not a table"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_cargo_toml_with_metadata() -> Result<()> {
        let content = r#"[package]
name = "test-project"
version = "0.1.0"
"#;
        let updated = update_cargo_toml_with_metadata(content)?;
        assert!(updated.contains(r"[package.metadata.deb]"));
        assert!(updated.contains(r"[package.metadata.generate-rpm]"));
        Ok(())
    }

    #[test]
    fn test_prepare_cargo_lib() -> Result<()> {
        let content = r#"[package]
name = "my-project"
"#;
        let updated = prepare_cargo_lib(content, "my-project", vec!["cdylib".into()], false)?;
        assert!(updated.contains(r#"name = "my_project_lib""#));
        Ok(())
    }
}
