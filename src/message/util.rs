pub struct Until<I>
where
    I: Iterator,
{
    iter: I,
    until: I::Item,
}

impl<I: Iterator> Until<I> {
    #[inline]
    pub fn new(iter: I, until: I::Item) -> Self {
        Self { iter, until }
    }
}

impl<I> Iterator for Until<I>
where
    I: Iterator,
    I::Item: PartialEq<I::Item> + Copy,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // self.iter.next().filter(|n| n != &self.until)
        let n = self.iter.next();
        // if n.contains(&self.until) {
        //     None
        // } else {
        //     n
        // }
        if let Some(b) = n {
            if b == self.until {
                return None;
            }
        }
        n
    }
}

pub trait UntilExt<I: Iterator> {
    fn until(self, until: I::Item) -> Until<I>;
}

impl<I> UntilExt<I> for I
where
    I: Iterator,
    I::Item: PartialEq<I::Item>,
{
    #[inline]
    fn until(self, until: I::Item) -> Until<I> {
        Until::new(self, until)
    }
}
