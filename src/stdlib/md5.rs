use crate::compiler::prelude::*;
use md5::Digest;

fn md5(value: Value) -> Resolved {
    let value = value.try_bytes()?;
    let digest = md5::Md5::digest(&value);
    let mut buf = [0u8; 32];
    hex::encode_to_slice(digest, &mut buf).expect("32 bytes");
    Ok(Value::Bytes(Bytes::copy_from_slice(&buf)))
}

#[derive(Clone, Copy, Debug)]
pub struct Md5;

impl Function for Md5 {
    fn identifier(&self) -> &'static str {
        "md5"
    }

    fn usage(&self) -> &'static str {
        "Calculates an md5 hash of the `value`."
    }

    fn category(&self) -> &'static str {
        Category::Cryptography.as_ref()
    }

    fn return_kind(&self) -> u16 {
        kind::BYTES
    }

    fn parameters(&self) -> &'static [Parameter] {
        const PARAMETERS: &[Parameter] = &[Parameter::required(
            "value",
            kind::BYTES,
            "The string to calculate the hash for.",
        )];
        PARAMETERS
    }

    fn examples(&self) -> &'static [Example] {
        &[example! {
            title: "Create md5 hash",
            source: r#"md5("foo")"#,
            result: Ok("acbd18db4cc2f85cedef654fccc4a4d8"),
        }]
    }

    fn compile(
        &self,
        _state: &state::TypeState,
        _ctx: &mut FunctionCompileContext,
        arguments: ArgumentList,
    ) -> Compiled {
        let value = arguments.required("value");

        Ok(Md5Fn { value }.as_expr())
    }
}

#[derive(Debug, Clone)]
struct Md5Fn {
    value: Box<dyn Expression>,
}

impl FunctionExpression for Md5Fn {
    fn resolve(&self, ctx: &mut Context) -> Resolved {
        let value = self.value.resolve(ctx)?;
        md5(value)
    }

    fn type_def(&self, _: &state::TypeState) -> TypeDef {
        TypeDef::bytes().infallible()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value;

    test_function![
        md5 => Md5;

        md5 {
            args: func_args![value: "foo"],
            want: Ok(value!("acbd18db4cc2f85cedef654fccc4a4d8")),
            tdef: TypeDef::bytes().infallible(),
        }

        md5_empty {
            args: func_args![value: ""],
            want: Ok(value!("d41d8cd98f00b204e9800998ecf8427e")),
            tdef: TypeDef::bytes().infallible(),
        }

        md5_sentence {
            args: func_args![value: "The quick brown fox jumps over the lazy dog"],
            want: Ok(value!("9e107d9d372bb6826bd81d3542a419d6")),
            tdef: TypeDef::bytes().infallible(),
        }
    ];
}
