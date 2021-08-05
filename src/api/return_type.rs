use std::collections::HashMap;

use jsonrpc_core::serde_json::{json, Map, Value};

/** idea of the return type abstraction, versus using json directly is that this way other formats can be easialy added in the future */
pub(crate) trait ReturnType {
    fn get_typescript_type() -> String;
    fn into_json_value(self) -> Value;

    /// whether this type should be created as ts type or contains custom types (as generic)
    fn makes_use_of_custom_ts_type() -> bool;

    /// get_typescript_type with custom type suport, when implementing it use the name your type should have in typescript
    /// (unless you implement it for generics, then you need to do the same as in `get_typescript_type`,
    /// only with the custom type names if availible instead of the actual type)
    fn get_typescript_type_with_custom_type_support() -> String;

    /// true for generics where `get_typescript_type_with_custom_type_support` does not return a type_name
    ///
    /// false when `get_typescript_type_with_custom_type_support`is the type_name
    const IS_WRAPPER: bool;
}

macro_rules! non_custom_return_type {
    () => {
        fn makes_use_of_custom_ts_type() -> bool {
            false
        }

        fn get_typescript_type_with_custom_type_support() -> String {
            Self::get_typescript_type()
        }

        const IS_WRAPPER: bool = false;
    };
}

#[macro_export]
macro_rules! custom_return_type {
    ($name:expr) => {
        fn makes_use_of_custom_ts_type() -> bool {
            true
        }

        fn get_typescript_type_with_custom_type_support() -> String {
            $name
        }

        const IS_WRAPPER: bool = false;
    };
}

impl ReturnType for () {
    fn get_typescript_type() -> String {
        "undefined".to_owned()
    }

    fn into_json_value(self) -> Value {
        Value::Null
    }

    non_custom_return_type!();
}

impl ReturnType for usize {
    fn get_typescript_type() -> String {
        "number".to_owned()
    }

    fn into_json_value(self) -> Value {
        json!(self)
    }

    non_custom_return_type!();
}

impl ReturnType for u32 {
    fn get_typescript_type() -> String {
        "number".to_owned()
    }
    fn into_json_value(self) -> Value {
        json!(self)
    }

    non_custom_return_type!();
}

impl ReturnType for String {
    fn get_typescript_type() -> String {
        "string".to_owned()
    }
    fn into_json_value(self) -> Value {
        json!(self)
    }

    non_custom_return_type!();
}

impl ReturnType for bool {
    fn get_typescript_type() -> String {
        "boolean".to_owned()
    }
    fn into_json_value(self) -> Value {
        Value::Bool(self)
    }

    non_custom_return_type!();
}

impl<K, V> ReturnType for HashMap<K, V>
where
    K: ReturnType,
    K: std::fmt::Display,
    V: ReturnType,
{
    fn get_typescript_type() -> String {
        format!(
            "{{ [key: {}]: {} }}",
            K::get_typescript_type(),
            V::get_typescript_type()
        )
    }
    fn into_json_value(mut self) -> Value {
        let mut map = Map::new();
        for (key, value) in self.drain() {
            map.insert(format!("{}", key), value.into_json_value());
        }
        Value::Object(map)
    }

    fn makes_use_of_custom_ts_type() -> bool {
        K::makes_use_of_custom_ts_type() || V::makes_use_of_custom_ts_type()
    }

    fn get_typescript_type_with_custom_type_support() -> String {
        format!(
            "{{ [key: {}]: {} }}",
            K::get_typescript_type_with_custom_type_support(),
            V::get_typescript_type_with_custom_type_support()
        )
    }

    const IS_WRAPPER: bool = true;
}

impl<T> ReturnType for Vec<T>
where
    T: ReturnType,
{
    fn get_typescript_type() -> String {
        let mut base = T::get_typescript_type();
        base.push_str("[]");
        base
    }
    fn into_json_value(self) -> Value {
        Value::Array(
            self.into_iter()
                .map(|item| item.into_json_value())
                .collect(),
        )
    }

    fn makes_use_of_custom_ts_type() -> bool {
        T::makes_use_of_custom_ts_type()
    }

    fn get_typescript_type_with_custom_type_support() -> String {
        let mut base = T::get_typescript_type_with_custom_type_support();
        base.push_str("[]");
        base
    }

    const IS_WRAPPER: bool = true;
}

impl<T> ReturnType for Option<T>
where
    T: ReturnType,
{
    fn get_typescript_type() -> String {
        let mut base = T::get_typescript_type();
        base.push_str("|null");
        base
    }
    fn into_json_value(self) -> Value {
        match self {
            Some(val) => val.into_json_value(),
            None => Value::Null,
        }
    }

    fn makes_use_of_custom_ts_type() -> bool {
        T::makes_use_of_custom_ts_type()
    }

    fn get_typescript_type_with_custom_type_support() -> String {
        let mut base = T::get_typescript_type_with_custom_type_support();
        base.push_str("|null");
        base
    }

    const IS_WRAPPER: bool = true;
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
