mod query_builder_ext;

use sqlx::{Database, Encode, Postgres, Type};
use std::fmt::Display;

pub trait QueryBuilderExt<'q, DB>
where
    DB: Database,
{
    fn push_and_bind_if<T>(
        &mut self,
        condition: bool,
        fragment: impl Display,
        bind_value: T,
    ) -> &mut Self
    where
        T: 'q + Encode<'q, Postgres> + Type<DB>;

    fn push_and_bind_if_with<B, T>(
        &mut self,
        condition: bool,
        fragment: impl Display,
        bind_value: B,
    ) -> &mut Self
    where
        B: Fn() -> T,
        T: 'q + Encode<'q, Postgres> + Type<DB>;

    fn push_separated_with<T, Sep>(
        &mut self,
        left_fragment: impl Display,
        separator: Sep,
        right_fragment: impl Display,
        values: impl IntoIterator<Item = T>,
    ) -> &mut Self
    where
        T: 'q + Encode<'q, Postgres> + Type<DB>,
        Sep: Display;
}
