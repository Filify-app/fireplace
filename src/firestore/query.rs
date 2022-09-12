// TODO: Include the limitations described here: https://firebase.google.com/docs/firestore/query-data/queries#query_limitations

/*
- [ ] < less than
- [ ] <= less than or equal to
- [ ] == equal to
- [ ] > greater than
- [ ] >= greater than or equal to
- [ ] != not equal to
- [ ] array-contains
- [ ] array-contains-any
- [ ] in
- [ ] not-in
*/

/// Represents a Firestore query operator used to test a field's value against
/// a given filter.
///
/// To see which query operators are supported by Firestore, see the [official Firestore documentation](https://firebase.google.com/docs/firestore/query-data/queries#query_operators).
pub trait QueryOperator<F> {
    fn test_against(&self, field: F) -> bool;
}

struct LessThan<T: Ord>(pub T);

impl<F: Ord> QueryOperator<F> for LessThan<F> {
    fn test_against(&self, field: F) -> bool {
        field < self.0
    }
}

struct LessThanOrEqualTo<T: Ord>(pub T);

impl<F: Ord> QueryOperator<F> for LessThanOrEqualTo<F> {
    fn test_against(&self, field: F) -> bool {
        field <= self.0
    }
}

struct ArrayContains<T: Eq>(pub T);

impl<T: Eq, F: IntoIterator<Item = T>> QueryOperator<F> for ArrayContains<T> {
    fn test_against(&self, field: F) -> bool {
        field.into_iter().any(|item| self.0 == item)
    }
}

struct In<'a, T: Eq>(&'a [T]);

impl<'a, F: Eq> QueryOperator<F> for In<'a, F> {
    fn test_against(&self, field: F) -> bool {
        self.0.contains(&field)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn less_than() {
        let query = LessThan(5);
        assert!(query.test_against(4));
        assert!(!query.test_against(5));
        assert!(!query.test_against(6));
    }

    #[test]
    fn less_than_or_equal_to() {
        let query = LessThanOrEqualTo(5);
        assert!(query.test_against(4));
        assert!(query.test_against(5));
        assert!(!query.test_against(6));

        let query = LessThanOrEqualTo("aaa");
        assert!(query.test_against("aa"));
        assert!(query.test_against("aaa"));
        assert!(!query.test_against("b"));
    }

    #[test]
    fn array_contains() {
        let query = ArrayContains(5);
        assert!(query.test_against(vec![1, 2, 3, 4, 5]));
        assert!(!query.test_against(vec![1, 2, 3, 4]));
        assert!(!query.test_against(vec![]));
    }
}
