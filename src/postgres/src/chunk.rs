pub trait Chunks: IntoIterator {
    fn chunks(self, size: usize) -> ChunksIter<Self::IntoIter, Self::Item>;
}

pub struct ChunksIter<I, T> {
    iter: I,
    buffer: Vec<T>,
    size: usize,
}

impl<I: IntoIterator> Chunks for I {
    fn chunks(self, size: usize) -> ChunksIter<I::IntoIter, I::Item> {
        ChunksIter {
            iter: self.into_iter(),
            buffer: Vec::with_capacity(size),
            size,
        }
    }
}

impl<I: Iterator> ChunksIter<I, I::Item> {
    pub fn next(&mut self) -> Option<&[I::Item]> {
        if !self.buffer.is_empty() {
            self.buffer.clear();
        }

        self.buffer.extend((&mut self.iter).take(self.size));

        if self.buffer.is_empty() {
            None
        } else {
            Some(&self.buffer)
        }
    }
}
