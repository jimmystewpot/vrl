use crate::compiler::prelude::*;

fn for_each<T>(value: Value, ctx: &mut Context, runner: &closure::Runner<T>) -> Resolved
where
    T: Fn(&mut Context) -> Resolved,
{
    match value {
        Value::Array(array) => {
            for (index, value) in array.into_iter().enumerate() {
                runner.run_index_value_owned(ctx, index, value)?;
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                runner.run_key_value_owned(ctx, key, value)?;
            }
        }
        _ => {}
    }

    Ok(Value::Null)
}

#[derive(Clone, Copy, Debug)]
pub struct ForEach;

impl Function for ForEach {
    fn identifier(&self) -> &'static str {
        "for_each"
    }

    fn usage(&self) -> &'static str {
        indoc! {"
            Iterate over a collection.

            This function currently *does not* support recursive iteration.

            The function uses the \"function closure syntax\" to allow reading
            the key/value or index/value combination for each item in the
            collection.

            The same scoping rules apply to closure blocks as they do for
            regular blocks. This means that any variable defined in parent scopes
            is accessible, and mutations to those variables are preserved,
            but any new variables instantiated in the closure block are
            unavailable outside of the block.

            See the examples below to learn about the closure syntax.
        "}
    }

    fn category(&self) -> &'static str {
        Category::Enumerate.as_ref()
    }

    fn return_kind(&self) -> u16 {
        kind::NULL
    }
    fn parameters(&self) -> &'static [Parameter] {
        const PARAMETERS: &[Parameter] = &[Parameter::required(
            "value",
            kind::OBJECT | kind::ARRAY,
            "The array or object to iterate.",
        )];
        PARAMETERS
    }

    fn examples(&self) -> &'static [Example] {
        &[
            example! {
                title: "Tally elements",
                source: indoc! {r#"
                    .tags = ["foo", "bar", "foo", "baz"]
                    tally = {}
                    for_each(array(.tags)) -> |_index, value| {
                        count = int(get!(tally, [value])) ?? 0
                        tally = set!(tally, [value], count + 1)
                    }
                    tally
                "#},
                result: Ok(r#"{"bar": 1, "baz": 1, "foo": 2}"#),
            },
            example! {
                title: "Iterate over an object",
                source: indoc! {r#"
                    count = 0
                    for_each({ "a": 1, "b": 2 }) -> |_key, value| {
                        count = count + value
                    }
                    count
                "#},
                result: Ok("3"),
            },
            example! {
                title: "Iterate over an array",
                source: indoc! {"
                    count = 0
                    for_each([1, 2, 3]) -> |index, value| {
                        count = count + index + value
                    }
                    count
                "},
                result: Ok("9"),
            },
        ]
    }

    fn compile(
        &self,
        _state: &state::TypeState,
        _ctx: &mut FunctionCompileContext,
        arguments: ArgumentList,
    ) -> Compiled {
        let value = arguments.required("value");
        let closure = arguments.required_closure()?;

        Ok(ForEachFn { value, closure }.as_expr())
    }

    fn closure(&self) -> Option<closure::Definition> {
        use closure::{Definition, Input, Output, Variable, VariableKind};

        Some(Definition {
            inputs: vec![Input {
                parameter_keyword: "value",
                kind: Kind::object(Collection::any()).or_array(Collection::any()),
                variables: vec![
                    Variable {
                        kind: VariableKind::TargetInnerKey,
                    },
                    Variable {
                        kind: VariableKind::TargetInnerValue,
                    },
                ],
                output: Output::Kind(Kind::any()),
                example: example! {
                    title: "iterate array",
                    source: "for_each([1, 2]) -> |index, value| { .foo = to_int!(.foo) + index + value }",
                    result: Ok("null"),
                },
            }],
            is_iterator: true,
        })
    }
}

#[derive(Debug, Clone)]
struct ForEachFn {
    value: Box<dyn Expression>,
    closure: Closure,
}

impl FunctionExpression for ForEachFn {
    fn resolve(&self, ctx: &mut Context) -> ExpressionResult<Value> {
        let value = self.value.resolve(ctx)?;
        let Closure {
            variables,
            block,
            block_type_def: _,
        } = &self.closure;
        let runner = closure::Runner::new(variables, |ctx| block.resolve(ctx));

        for_each(value, ctx, &runner)
    }

    fn type_def(&self, _ctx: &state::TypeState) -> TypeDef {
        TypeDef::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::Ident;
    use std::cell::RefCell;

    fn ident(s: &str) -> Ident {
        Ident::from(s.to_string())
    }

    fn test_context() -> (Value, state::RuntimeState, TimeZone) {
        (
            Value::Null,
            state::RuntimeState::default(),
            TimeZone::default(),
        )
    }

    #[test]
    fn test_for_each_empty_array() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let count = RefCell::new(0);
        let variables = [ident("i"), ident("v")];
        let runner = closure::Runner::new(&variables, |_ctx| {
            *count.borrow_mut() += 1;
            Ok(Value::Null)
        });

        let res = for_each(Value::Array(vec![]), &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(*count.borrow(), 0);
    }

    #[test]
    fn test_for_each_empty_object() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let count = RefCell::new(0);
        let variables = [ident("k"), ident("v")];
        let runner = closure::Runner::new(&variables, |_ctx| {
            *count.borrow_mut() += 1;
            Ok(Value::Null)
        });

        let res = for_each(Value::Object(ObjectMap::new()), &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(*count.borrow(), 0);
    }

    #[test]
    fn test_for_each_array_index_and_value() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let visited = RefCell::new(Vec::new());
        let variables = [ident("i"), ident("v")];
        let runner = closure::Runner::new(&variables, |ctx| {
            let i = ctx.state().variable(&ident("i")).cloned().unwrap();
            let v = ctx.state().variable(&ident("v")).cloned().unwrap();
            visited.borrow_mut().push((i, v));
            Ok(Value::Null)
        });

        let array = Value::Array(vec![Value::from("first"), Value::from("second")]);
        let res = for_each(array, &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(
            visited.into_inner(),
            vec![
                (Value::Integer(0), Value::from("first")),
                (Value::Integer(1), Value::from("second")),
            ]
        );
    }

    #[test]
    fn test_for_each_object_key_and_value() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let visited = RefCell::new(Vec::new());
        let variables = [ident("k"), ident("v")];
        let runner = closure::Runner::new(&variables, |ctx| {
            let k = ctx.state().variable(&ident("k")).cloned().unwrap();
            let v = ctx.state().variable(&ident("v")).cloned().unwrap();
            visited.borrow_mut().push((k, v));
            Ok(Value::Null)
        });

        let mut map = ObjectMap::new();
        map.insert("a".into(), Value::Integer(10));
        map.insert("b".into(), Value::Integer(20));

        let res = for_each(Value::Object(map), &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(
            visited.into_inner(),
            vec![
                (Value::from("a"), Value::Integer(10)),
                (Value::from("b"), Value::Integer(20)),
            ]
        );
    }

    #[test]
    fn test_for_each_non_collection() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let count = RefCell::new(0);
        let variables = [ident("k"), ident("v")];
        let runner = closure::Runner::new(&variables, |_ctx| {
            *count.borrow_mut() += 1;
            Ok(Value::Null)
        });

        let non_collections = vec![
            Value::Null,
            Value::Integer(42),
            Value::from("string"),
            Value::Boolean(true),
        ];

        for val in non_collections {
            let res = for_each(val, &mut ctx, &runner);
            assert_eq!(res, Ok(Value::Null));
        }
        assert_eq!(*count.borrow(), 0);
    }

    #[test]
    fn test_for_each_early_return_in_closure() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let count = RefCell::new(0);
        let variables = [ident("k"), ident("v")];
        let runner = closure::Runner::new(&variables, |_ctx| {
            *count.borrow_mut() += 1;
            Err(ExpressionError::Return {
                span: Span::new(0, 0),
                value: Value::from(42),
            })
        });

        let mut map = ObjectMap::new();
        map.insert("a".into(), Value::Integer(1));
        map.insert("b".into(), Value::Integer(2));

        let res = for_each(Value::Object(map), &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn test_for_each_error_halts_iteration() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let count = RefCell::new(0);
        let variables = [ident("i"), ident("v")];
        let runner = closure::Runner::new(&variables, |_ctx| {
            *count.borrow_mut() += 1;
            Err(ExpressionError::from("abort error"))
        });

        let array = Value::Array(vec![Value::from(1), Value::from(2), Value::from(3)]);
        let res = for_each(array, &mut ctx, &runner);
        assert!(res.is_err());
        assert_eq!(*count.borrow(), 1);
    }

    #[test]
    fn test_for_each_array_wildcard_index() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let visited = RefCell::new(Vec::new());
        // First parameter is wildcard `_` (represented as empty ident)
        let variables = [ident(""), ident("v")];
        let runner = closure::Runner::new(&variables, |ctx| {
            assert!(ctx.state().variable(&ident("")).is_none());
            let v = ctx.state().variable(&ident("v")).cloned().unwrap();
            visited.borrow_mut().push(v);
            Ok(Value::Null)
        });

        let array = Value::Array(vec![Value::from("alpha"), Value::from("beta")]);
        let res = for_each(array, &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(
            visited.into_inner(),
            vec![Value::from("alpha"), Value::from("beta")]
        );
    }

    #[test]
    fn test_for_each_object_wildcard_key() {
        let (mut target, mut runtime_state, tz) = test_context();
        let mut ctx = Context::new(&mut target, &mut runtime_state, &tz);

        let visited = RefCell::new(Vec::new());
        // Key parameter is wildcard `_`
        let variables = [ident("_"), ident("v")];
        let runner = closure::Runner::new(&variables, |ctx| {
            assert!(ctx.state().variable(&ident("_")).is_none());
            let v = ctx.state().variable(&ident("v")).cloned().unwrap();
            visited.borrow_mut().push(v);
            Ok(Value::Null)
        });

        let mut map = ObjectMap::new();
        map.insert("k1".into(), Value::Integer(100));
        map.insert("k2".into(), Value::Integer(200));

        let res = for_each(Value::Object(map), &mut ctx, &runner);
        assert_eq!(res, Ok(Value::Null));
        assert_eq!(
            visited.into_inner(),
            vec![Value::Integer(100), Value::Integer(200)]
        );
    }
}
