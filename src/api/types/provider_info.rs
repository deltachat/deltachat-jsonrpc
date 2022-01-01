use deltachat::provider::Provider;
use jsonrpc_core::serde_json::{json, Value};
use num_traits::cast::ToPrimitive;

use super::return_type::*;
use crate::custom_return_type;

pub struct ProviderInfo {
    pub before_login_hint: String,
    pub overview_page: String,
    pub status: u32, // in reality this is an enum, but for simlicity and because it gets converted into a number anyway, we use an u32 here.
}

impl ProviderInfo {
    pub fn from_dc_type(provider: Option<&Provider>) -> Option<Self> {
        provider.map(|p| ProviderInfo {
            before_login_hint: p.before_login_hint.to_owned(),
            overview_page: p.overview_page.to_owned(),
            status: p.status.to_u32().unwrap(),
        })
    }
}

impl ReturnType for ProviderInfo {
    fn get_typescript_type() -> String {
        "{ before_login_hint: string, overview_page: string, status: 1 | 2 | 3 }".to_owned()
    }

    fn into_json_value(self) -> Value {
        json!({
            "before_login_hint": self.before_login_hint,
            "overview_page": self.overview_page,
            "status": self.status
        })
    }

    custom_return_type!("ProviderInfo_Type".to_owned());
}
