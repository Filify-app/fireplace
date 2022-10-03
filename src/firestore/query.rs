// TODO: Include the limitations described here: https://firebase.google.com/docs/firestore/query-data/queries#query_limitations
// TODO: Add doc comment to each implementor

/*
- [x] < less than
- [ ] <= less than or equal to
- [x] == equal to
- [ ] > greater than
- [ ] >= greater than or equal to
- [ ] != not equal to
- [x] array-contains
- [ ] array-contains-any
- [ ] in
- [ ] not-in
*/

use firestore_grpc::v1::{
    structured_query::{
        composite_filter::Operator as CompositeFilterOperator,
        field_filter::Operator as FieldFilterOperator, filter::FilterType as GrpcFilterType,
        CompositeFilter as GrpcCompositeFilter, FieldFilter as GrpcFieldFilter,
        Filter as GrpcFilter,
    },
    Value,
};
use serde::Serialize;

use crate::error::FirebaseError;

use super::serde::serialize_to_value_type;

/// Represents a Firestore query operator used to test a field's value against
/// a given filter.
///
/// It will be possible to put any arbitrary struct or value into a query
/// operator (as long as the value satisfies the trait bounds), but you will
/// likely want to stick with primitive types like integers, strings, or lists
/// of the two.
///
/// You will see that many of the implementors of this trait will impose
/// trait bounds on the type parameter `T`. This is only done to help you catch
/// potential logic errors in your program. For example, `LessThan` requires
/// that `T` implements `Ord` - but the value will not actually be compared
/// client-side but only on the server-side by Firestore. Therefore your
/// implementation of `Ord` will not affect the filtering behavior.
///
/// To see which query operators are supported by Firestore, see the [official Firestore documentation](https://firebase.google.com/docs/firestore/query-data/queries#query_operators).
pub trait QueryOperator<T: Serialize> {
    /// Returns the value that the document field will be checked against.
    fn get_value(&self) -> &T;

    /// Returns the Firestore field filter operator code that represents which
    /// filter operation will be applied to the value by Firestore.
    fn get_operator_code(&self) -> FieldFilterOperator;
}

pub struct LessThan<T: Ord + Serialize>(pub T);

impl<T: Ord + Serialize> QueryOperator<T> for LessThan<T> {
    fn get_value(&self) -> &T {
        &self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::LessThan
    }
}

pub struct EqualTo<T: Eq + Serialize>(pub T);

impl<T: Eq + Serialize> QueryOperator<T> for EqualTo<T> {
    fn get_value(&self) -> &T {
        &self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::Equal
    }
}

pub struct ArrayContains<T: Eq + Serialize>(pub T);

impl<T: Eq + Serialize> QueryOperator<T> for ArrayContains<T> {
    fn get_value(&self) -> &T {
        &self.0
    }

    fn get_operator_code(&self) -> FieldFilterOperator {
        FieldFilterOperator::ArrayContains
    }
}

pub fn filter<T: Serialize>(
    field: impl Into<String>,
    check_against: impl QueryOperator<T>,
) -> Result<Filter, FirebaseError> {
    let field_filter = create_field_filter(field.into(), check_against)?;
    Ok(Filter::Single(field_filter))
}

#[derive(PartialEq, Debug)]
pub enum Filter {
    Composite(Vec<FieldFilter>),
    Single(FieldFilter),
}

#[derive(PartialEq, Debug)]
pub struct FieldFilter {
    field: String,
    op: FieldFilterOperator,
    value: Value,
}

impl Filter {
    pub fn and<T: Serialize>(
        self,
        field: impl Into<String>,
        check_against: impl QueryOperator<T>,
    ) -> Result<Self, FirebaseError> {
        let other_field_filter = create_field_filter(field.into(), check_against)?;

        let new_filter = match self {
            Filter::Composite(mut filters) => {
                filters.push(other_field_filter);
                Filter::Composite(filters)
            }
            Filter::Single(filter) => Filter::Composite(vec![filter, other_field_filter]),
        };

        Ok(new_filter)
    }
}

fn create_field_filter<T: Serialize>(
    field: String,
    query_op: impl QueryOperator<T>,
) -> Result<FieldFilter, FirebaseError> {
    let val = query_op.get_value();
    let value_type = serialize_to_value_type(val)?;
    let firestore_value = Value {
        value_type: Some(value_type),
    };

    Ok(FieldFilter {
        field,
        op: query_op.get_operator_code(),
        value: firestore_value,
    })
}

impl From<Filter> for GrpcFilter {
    fn from(filter: Filter) -> Self {
        let filter_type = match filter {
            Filter::Single(filter) => filter.into(),
            Filter::Composite(filters) => {
                let f = filters.into_iter().map(Into::into).collect();
                GrpcFilterType::CompositeFilter(GrpcCompositeFilter {
                    op: CompositeFilterOperator::And as i32,
                    filters: f,
                })
            }
        };

        Self {
            filter_type: Some(filter_type),
        }
    }
}

impl From<FieldFilter> for GrpcFilterType {
    fn from(field_filter: FieldFilter) -> Self {
        GrpcFilterType::FieldFilter(GrpcFieldFilter {
            field: Some(firestore_grpc::v1::structured_query::FieldReference {
                field_path: field_filter.field,
            }),
            op: field_filter.op as i32,
            value: Some(field_filter.value),
        })
    }
}

impl From<FieldFilter> for GrpcFilter {
    fn from(field_filter: FieldFilter) -> Self {
        Self {
            filter_type: Some(field_filter.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use firestore_grpc::v1::{structured_query, value::ValueType};

    use super::*;

    #[test]
    fn combine_operators() {
        assert_eq!(
            (|| filter("age", LessThan(42))?.and("name", EqualTo("Bob")))().unwrap(),
            Filter::Composite(vec![
                FieldFilter {
                    field: "age".to_string(),
                    op: FieldFilterOperator::LessThan,
                    value: Value {
                        value_type: Some(ValueType::IntegerValue(42)),
                    },
                },
                FieldFilter {
                    field: "name".to_string(),
                    op: FieldFilterOperator::Equal,
                    value: Value {
                        value_type: Some(ValueType::StringValue("Bob".to_string())),
                    },
                },
            ])
        );
    }

    #[test]
    fn into_grpc_filter() {
        let f: structured_query::Filter = filter("age", LessThan(42)).unwrap().into();

        assert_eq!(
            f,
            structured_query::Filter {
                filter_type: Some(structured_query::filter::FilterType::FieldFilter(
                    structured_query::FieldFilter {
                        field: Some(structured_query::FieldReference {
                            field_path: "age".to_string(),
                        }),
                        op: structured_query::field_filter::Operator::LessThan as i32,
                        value: Some(Value {
                            value_type: Some(ValueType::IntegerValue(42)),
                        }),
                    }
                )),
            }
        )
    }
}
