pub mod dashboard;
pub mod system_status;

use crate::modules::dashboard::DashboardModule;
use crate::modules::system_status::SystemStatusModule;
use crate::Module;

pub fn register_modules() -> Vec<Box<dyn Module>> {
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    modules.push(Box::new(DashboardModule::new()));
    modules.push(Box::new(SystemStatusModule::new()));
    modules
}
