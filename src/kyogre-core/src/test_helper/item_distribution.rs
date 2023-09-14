use std::cmp::Ordering;

#[derive(Debug)]
pub struct ItemDistribution {
    num_items: usize,
    distribution: ItemsPerElement,
}

#[derive(Debug)]
enum ItemsPerElement {
    Equal(usize),
    EqualWithRemainder { per: usize, remainder: usize },
    Uneven,
}

impl ItemDistribution {
    pub fn new(num_elements: usize, num_items: usize) -> ItemDistribution {
        let val = match num_elements.cmp(&num_items) {
            Ordering::Equal => ItemsPerElement::Equal(1),
            Ordering::Less => ItemsPerElement::Uneven,
            Ordering::Greater => {
                if num_elements % num_items == 0 {
                    ItemsPerElement::Equal(num_elements / num_items)
                } else {
                    let remainder = (num_elements % num_items) + (num_elements / num_items);
                    ItemsPerElement::EqualWithRemainder {
                        per: num_elements / num_items,
                        remainder,
                    }
                }
            }
        };
        ItemDistribution {
            num_items,
            distribution: val,
        }
    }
    pub fn num_elements(&self, item_index: usize) -> usize {
        match self.distribution {
            ItemsPerElement::Equal(v) => v,
            ItemsPerElement::EqualWithRemainder { per, remainder } => {
                if item_index == self.num_items - 1 {
                    per + remainder
                } else {
                    per
                }
            }
            ItemsPerElement::Uneven => {
                if item_index >= self.num_items - 1 {
                    0
                } else {
                    1
                }
            }
        }
    }
}
