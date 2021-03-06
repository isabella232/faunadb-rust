use crate::{
    expr::{Expr, IndexPermission, Object},
    query::Query,
};
use std::borrow::Cow;

boxed_query!(CreateIndex);

/// The `CreateIndex` function adds a new index to the cluster with the specified
/// parameters.
///
/// After the transaction containing the `CreateIndex` is completed,
/// the index is immediately available for reads. (The index may not be used in
/// the transaction it was created, and it may not be created in the same
/// transaction as its source class(es).) The index may return incomplete
/// results until it is fully built and marked as active. FaunaDB builds the
/// index asynchronously by scanning over relevant instance objects of the
/// source class(es).
///
/// Read the
/// [docs](https://docs.fauna.com/fauna/current/reference/queryapi/write/createindex)
#[derive(Debug, Serialize, Clone)]
pub struct CreateIndex<'a> {
    create_index: IndexParams<'a>,
}

impl<'a> CreateIndex<'a> {
    pub fn new(params: IndexParams<'a>) -> Self {
        Self {
            create_index: params,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[doc(hidden)]
pub struct IndexField<'a>(Vec<Cow<'a, str>>);

#[derive(Debug, Serialize, Clone)]
#[doc(hidden)]
pub struct IndexBinding<'a>(Cow<'a, str>);

#[derive(Debug, Serialize, Clone)]
#[doc(hidden)]
pub enum TermObject<'a> {
    #[serde(rename = "field")]
    Field(IndexField<'a>),
    #[serde(rename = "binding")]
    Binding(IndexBinding<'a>),
}

/// Term objects describe the fields used to locate entries in the index.
///
/// If multiple terms are provided, instances missing a value will emit a Null
/// term in the index for that field.
///
/// Read the
/// [docs](https://docs.fauna.com/fauna/current/reference/indexconfig#term-objects)
#[derive(Debug, Serialize, Clone)]
pub struct Term<'a> {
    object: TermObject<'a>,
}

#[derive(Debug, Serialize, Clone)]
#[doc(hidden)]
pub struct ValueObject<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<IndexField<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    binding: Option<IndexBinding<'a>>,
    reverse: bool,
}

/// Value objects describe the data covered by the index, which are included in
/// query results on the index and control ordering of entries having the same
/// terms.
///
/// By default, indexes cover only the refs of included instances.
///
/// Read the
/// [docs](https://docs.fauna.com/fauna/current/reference/indexconfig#value-objects)
#[derive(Debug, Serialize, Clone)]
pub struct IndexValue<'a> {
    object: ValueObject<'a>,
}

#[derive(Debug, Serialize, Clone)]
#[doc(hidden)]
pub struct IndexParamsInternal<'a> {
    name: Cow<'a, str>,
    source: Expr<'a>,
    active: bool,
    unique: bool,
    serialized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    terms: Option<Vec<Term<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<IndexValue<'a>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partitions: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<IndexPermission<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Expr<'a>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct IndexParams<'a> {
    object: IndexParamsInternal<'a>,
}

impl<'a> Term<'a> {
    /// The path of the field within an instance to be indexed.
    pub fn field<T>(path: Vec<T>) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        let field = IndexField(path.into_iter().map(Into::into).collect());

        Self {
            object: TermObject::Field(field),
        }
    }

    /// The name of a binding from a source object.
    pub fn binding<T>(name: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        let binding = IndexBinding(name.into());

        Self {
            object: TermObject::Binding(binding),
        }
    }
}

impl<'a> IndexValue<'a> {
    /// The path of the field within an instance to be indexed.
    pub fn field<T>(path: Vec<T>) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        let field = IndexField(path.into_iter().map(Into::into).collect());

        Self {
            object: ValueObject {
                field: Some(field),
                binding: None,
                reverse: false,
            },
        }
    }

    /// The name of a binding from a source object.
    pub fn binding<T>(name: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        let binding = IndexBinding(name.into());

        Self {
            object: ValueObject {
                field: None,
                binding: Some(binding),
                reverse: false,
            },
        }
    }

    /// If set, the sort of the field's value is reversed.
    pub fn reverse(&mut self) -> &mut Self {
        self.object.reverse = true;
        self
    }
}

impl<'a> IndexParams<'a> {
    /// The name cannot be `events`, `sets`, `self`, `instances` or `_`.
    ///
    /// The source must evaluate to a class `Ref`.
    pub fn new<S>(name: S, source: impl Into<Expr<'a>>) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        Self {
            object: IndexParamsInternal {
                name: name.into(),
                source: source.into(),
                active: false,
                unique: false,
                serialized: false,
                terms: None,
                values: None,
                partitions: None,
                permissions: None,
                data: None,
            },
        }
    }

    /// If set, avoids building the index from relevant instances.
    pub fn active(&mut self) -> &mut Self {
        self.object.active = true;
        self
    }

    /// If set, maintains a unique constraint on combined `terms` and `values`.
    pub fn unique(&mut self) -> &mut Self {
        self.object.unique = true;
        self
    }

    /// If set, writes to the index are serialized with concurrent reads and
    /// writes.
    pub fn serialized(&mut self) -> &mut Self {
        self.object.serialized = true;
        self
    }

    /// An array of [Term objects](struct.Term.html) describing the fields to be
    /// indexed.
    pub fn terms(&mut self, terms: Vec<Term<'a>>) -> &mut Self {
        self.object.terms = Some(terms);
        self
    }

    /// An array of [Value objects](struct.IndexValue.html) describing the fields to be
    /// covered.
    pub fn values(&mut self, values: Vec<IndexValue<'a>>) -> &mut Self {
        self.object.values = Some(values);
        self
    }

    /// The number of sub-partitions within each term.
    pub fn partitions(&mut self, partitions: u16) -> &mut Self {
        self.object.partitions = Some(partitions);
        self
    }

    /// Indicates who is allowed to read the index.
    pub fn permissions(&mut self, permissions: IndexPermission<'a>) -> &mut Self {
        self.object.permissions = Some(permissions);
        self
    }

    /// The user-defined metadata for the index. It is provided for the
    /// developer to store information at the index level.
    pub fn data(&mut self, data: Object<'a>) -> &mut Self {
        self.object.data = Some(Expr::from(data));
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use serde_json::{self, json};

    #[test]
    fn test_create_index() {
        let mut permission = IndexPermission::default();
        permission.read(Level::public());

        let mut params = IndexParams::new("meows", Ref::class("cats"));
        params.permissions(permission);

        let age_term = Term::field(vec!["data", "age"]);
        let name_term = Term::binding("cats_name");

        params.terms(vec![age_term, name_term]);

        let name_value = IndexValue::field(vec!["data", "name"]);

        let mut age_value = IndexValue::binding("cats_age");
        age_value.reverse();

        params.values(vec![age_value, name_value]);

        let query = Query::from(CreateIndex::new(params));
        let serialized = serde_json::to_value(&query).unwrap();

        let expected = json!({
            "create_index": {
                "object": {
                    "active": false,
                    "name": "meows",
                    "permissions": {
                        "object": {
                            "read": "public",
                        }
                    },
                    "serialized": false,
                    "source": {
                        "@ref": {
                            "class": {
                                "@ref": {
                                    "id": "classes",
                                },
                            },
                            "id": "cats",
                        },
                    },
                    "terms": [
                        {
                            "object": {
                                "field": ["data", "age"],
                            }
                        },
                        {
                            "object": {
                                "binding": "cats_name",
                            }
                        },
                    ],
                    "unique": false,
                    "values": [
                        {
                            "object": {
                                "binding": "cats_age",
                                "reverse": true,
                            }
                        },
                        {
                            "object": {
                                "field": ["data", "name"],
                                "reverse": false,
                            }
                        },
                    ]
                }
            }
        });

        assert_eq!(expected, serialized);
    }
}
