pub mod dashboard;

use crate::modules::dashboard::DashboardModule;
use crate::Module;

pub fn register_modules() -> Vec<Box<dyn Module>> {
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    modules.push(Box::new(DashboardModule::new()));
    modules
}
