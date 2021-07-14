use jsonrpc_core::serde_json::{json, Value};
use jsonrpc_core::Result;

/** idea of the return type abstraction, versus using json directly is that this way other formats can be easialy added in the future */
pub(crate) trait ReturnType {
    fn get_typescript_type() -> String;
    fn into_json_value(self) -> Value;
}

impl ReturnType for () {
    fn get_typescript_type() -> String {
        "undefined".to_owned()
    }

    fn into_json_value(self) -> Value {
        Value::Null
    }
}

impl ReturnType for usize {
    fn get_typescript_type() -> String {
        "number".to_owned()
    }

    fn into_json_value(self) -> Value {
        json!(self)
    }
}

impl ReturnType for u32 {
    fn get_typescript_type() -> String {
        "number".to_owned()
    }
    fn into_json_value(self) -> Value {
        json!(self)
    }
}

impl ReturnType for String {
    fn get_typescript_type() -> String {
        "string".to_owned()
    }
    fn into_json_value(self) -> Value {
        json!(self)
    }
}

impl<T> ReturnType for Vec<T>
where
    T: ReturnType,
{
    fn get_typescript_type() -> String {
        "string".to_owned()
    }
    fn into_json_value(self) -> Value {
        Value::Array(
            self.into_iter()
                .map(|item| item.into_json_value())
                .collect(),
        )
    }
}

pub(crate) fn result_convert_anyhow_into_json_rpc<T>(
    result: anyhow::Result<T>,
) -> jsonrpc_core::Result<Value>
where
    T: ReturnType,
{
    match result {
        Ok(val) => jsonrpc_core::Result::Ok(val.into_json_value()),
        Err(err) => jsonrpc_core::Result::Err(jsonrpc_core::Error {
            code: jsonrpc_core::ErrorCode::ServerError(1),
            message: format!("{}", err),
            data: None,
        }),
    }
}
