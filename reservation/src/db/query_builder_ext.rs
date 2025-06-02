use std::fmt::Display;

use sqlx::{Encode, Postgres, QueryBuilder, Type};

use super::QueryBuilderExt;

impl<'args> QueryBuilderExt<'args, Postgres> for QueryBuilder<'args, Postgres> {
    fn push_and_bind_if<T>(
        &mut self,
        condition: bool,
        fragment: impl Display,
        bind_value: T,
    ) -> &mut Self
    where
        T: 'args + Encode<'args, Postgres> + Type<Postgres>,
    {
        if condition {
            return self.push(fragment).push_bind(bind_value);
        }
        self
    }

    fn push_and_bind_if_with<B, T>(
        &mut self,
        condition: bool,
        fragment: impl Display,
        bind_value: B,
    ) -> &mut Self
    where
        B: Fn() -> T,
        T: 'args + Encode<'args, Postgres> + Type<Postgres>,
    {
        if condition {
            return self.push(fragment).push_bind(bind_value());
        }
        self
    }

    fn push_separated_with<T, Sep>(
        &mut self,
        left_fragment: impl Display,
        separator: Sep,
        right_fragment: impl Display,
        values: impl IntoIterator<Item = T>,
    ) -> &mut Self
    where
        T: 'args + Encode<'args, Postgres> + Type<Postgres>,
        Sep: Display,
    {
        self.push(left_fragment);
        let mut separated = self.separated(separator);
        for value in values.into_iter() {
            separated.push_bind(value);
        }
        separated.push_unseparated(right_fragment);
        self
    }
}
