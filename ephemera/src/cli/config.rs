use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;
use log::{error, info};
use toml::{Table, Value};

use crate::config::Configuration;

#[derive(Debug, Clone, Parser)]
pub struct UpdateConfigCmd {
    #[clap(long)]
    pub config_path: String,
    #[clap(long)]
    pub property: String,
    #[clap(long)]
    pub value: String,
}

impl UpdateConfigCmd {
    /// # Panics
    /// Panics if the config file does not exist.
    pub fn execute(self) {
        let path: PathBuf = self.config_path.clone().into();
        match Configuration::try_load(path.clone()) {
            Ok(_) => {
                info!("Updating config: {:?}", self);

                let toml_str = fs::read_to_string(path.clone()).unwrap();
                let table = toml_str.parse::<Table>().unwrap();

                let keys = self.property.split('.').collect::<Vec<&str>>();

                if !table.contains_key(keys[0]) {
                    println!("Key '{}' does not exist", keys[0]);
                    return;
                }

                let mut visitor = ConfigVisitor::new(keys, self.value);
                let table = visitor.process(table);

                let toml_str = toml::to_string(&table).unwrap();
                fs::write(path, toml_str).unwrap();
            }
            Err(err) => {
                error!("Error loading configuration file: {err:?}");
            }
        }
    }
}

struct ConfigVisitor<'a> {
    keys: Vec<&'a str>,
    value: String,
    in_array: bool,
    data_in_array: Vec<Value>,
}

impl<'a> ConfigVisitor<'a> {
    fn new(keys: Vec<&'a str>, value: String) -> Self {
        Self {
            keys,
            value,
            in_array: false,
            data_in_array: vec![],
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn process(&mut self, table: Table) -> Table {
        let key = self.keys[0];
        let value = table.get(key).unwrap();
        self.process_value(table.clone(), value.clone(), 0)
    }

    fn process_value(&mut self, mut table: Table, value: Value, key_index: usize) -> Table {
        let root_key = self.keys[key_index];
        match value {
            Value::String(str) => {
                println!("Old value: {str}",);
                let value = String::from(&self.value);
                println!("New value: {value}",);
                table.insert(root_key.to_string(), Value::String(value));
            }
            Value::Integer(i) => {
                println!("Old value: {i}",);
                let value = i64::from_str(&self.value).unwrap();
                println!("New value: {value}",);
                table.insert(root_key.to_string(), Value::Integer(value));
            }
            Value::Float(f) => {
                println!("Old value: {f}",);
                let value = f64::from_str(&self.value).unwrap();
                println!("New value: {value}",);
                table.insert(root_key.to_string(), Value::Float(value));
            }
            Value::Boolean(b) => {
                println!("Old value: {b}",);
                let value = bool::from_str(&self.value).unwrap();
                println!("New value: {value}",);
                table.insert(root_key.to_string(), Value::Boolean(value));
            }
            Value::Datetime(_) => {
                println!("Datetime not supported");
            }
            Value::Array(ar) => {
                self.in_array = true;
                for value in ar {
                    self.process_value(table.clone(), value, key_index);
                }
                table.remove(root_key);
                table.insert(
                    root_key.to_string(),
                    Value::Array(self.data_in_array.clone()),
                );
                self.data_in_array.clear();
                self.in_array = false;
            }
            Value::Table(t) => {
                let value = t.get(self.keys[key_index + 1]).cloned().unwrap();
                let updated_table = self.process_value(t, value, key_index + 1);
                if self.in_array {
                    self.data_in_array.push(updated_table.into());
                    return table.clone();
                }
                table.insert(root_key.to_string(), updated_table.into());
            }
        }
        table
    }
}
