pub mod dashboard;
pub mod network;
pub mod system_info;

use crate::modules::dashboard::DashboardModule;
use crate::modules::network::NetworkModule;
use crate::modules::system_info::SystemInfoModule;
use crate::Module;

pub fn register_modules() -> Vec<Box<dyn Module>> {
    let mut modules: Vec<Box<dyn Module>> = Vec::new();
    modules.push(Box::new(DashboardModule::new()));
    modules.push(Box::new(SystemInfoModule::new()));
    modules.push(Box::new(NetworkModule::new()));
    modules
}
