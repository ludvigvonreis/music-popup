use crate::view::{AnyView, IntoView};

pub trait IntoViewIter {
	fn into_view_iter(self) -> impl Iterator<Item = AnyView>;
}

impl IntoViewIter for () {
    fn into_view_iter(self) -> impl Iterator<Item = AnyView> {
        std::iter::empty()
    }
}

impl<V: IntoView, const N: usize> IntoViewIter for [V; N] {
    fn into_view_iter(self) -> impl Iterator<Item = AnyView> {
        self.into_iter().map(|v| v.into_any())
    }
}

macro_rules! impl_into_view_iter_for_tuple {
    ($($t:ident),+; $($idx:tt),+) => {
        impl<$($t: IntoView),+> IntoViewIter for ($($t,)+) {
            fn into_view_iter(self) -> impl Iterator<Item = AnyView> {
                [$(self.$idx.into_any()),+].into_iter()
            }
        }
    };
}

impl_into_view_iter_for_tuple!(A; 0);
impl_into_view_iter_for_tuple!(A, B; 0, 1);
impl_into_view_iter_for_tuple!(A, B, C; 0, 1, 2);
impl_into_view_iter_for_tuple!(A, B, C, D; 0, 1, 2, 3);
impl_into_view_iter_for_tuple!(A, B, C, D, E; 0, 1, 2, 3, 4);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F; 0, 1, 2, 3, 4, 5);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G; 0, 1, 2, 3, 4, 5, 6);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H; 0, 1, 2, 3, 4, 5, 6, 7);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I; 0, 1, 2, 3, 4, 5, 6, 7, 8);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);
impl_into_view_iter_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P; 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);