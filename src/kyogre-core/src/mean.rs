pub trait Mean<T> {
    fn mean(self) -> Option<T>;
}

macro_rules! int_mean {
    ($($num:ty)*) => ($(
        impl<I> Mean<$num> for I
        where
            I: Iterator<Item = $num>
        {
            fn mean(mut self) -> Option<$num> {
                let first = self.next()?;
                let (sum, count) = self.fold((first, 1), |(sum, count), next| {
                    (sum + next, count + 1)
                });
                Some(sum / count)
            }
        }
    )*)
}

macro_rules! float_mean {
    ($($num:ty)*) => ($(
        impl<I> Mean<$num> for I
        where
            I: Iterator<Item = $num>
        {
            fn mean(mut self) -> Option<$num> {
                let first = self.next()?;
                let (sum, count) = self.fold((first, 1), |(sum, count), next| {
                    (sum + next, count + 1)
                });
                Some(sum / count as $num)
            }
        }
    )*)
}

int_mean! { i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize }
float_mean! { f32 f64 }
