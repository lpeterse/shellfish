use super::*;
use std::slice::IterMut;

pub struct ChannelMap {
    capacity: usize,
    elements: Vec<Option<ChannelState>>,
}

impl ChannelMap {
    pub fn new(capacity: usize) -> Self {
        ChannelMap {
            capacity,
            elements: Vec::with_capacity(1),
        }
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for e in &self.elements {
            if e.is_some() {
                len += 1
            };
        }
        len
    }

    pub fn get(&mut self, i: usize) -> Option<&ChannelState> {
        match self.elements.get(i) {
            None => None,
            Some(None) => None,
            Some(Some(e)) => Some(&e),
        }
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut ChannelState> {
        match self.elements.get_mut(i) {
            None => None,
            Some(None) => None,
            Some(Some(t)) => Some(t),
        }
    }

    pub fn free_key(&self) -> Option<usize> {
        Some(0) // FIXME
    }

    pub fn insert(&mut self, _key: usize, t: ChannelState) {
        // FIXME
        self.elements.push(Some(t));
    }

    pub fn remove(&mut self, i: usize) -> Option<ChannelState> {
        // TODO: shrink vector
        if i < self.elements.len() {
            std::mem::replace(&mut self.elements[i], None)
        } else {
            None
        }
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        for element in &mut self.elements {
            match element {
                None => (),
                Some(_) => match std::mem::replace(element, None) {
                    Some(c) => c.terminate(e),
                    None => (),
                },
            }
        }
    }
}

impl<'a> IntoIterator for &'a mut ChannelMap {
    type Item = &'a mut ChannelState;
    type IntoIter = LowestKeyMapIterator<'a>;

    fn into_iter(self) -> LowestKeyMapIterator<'a> {
        LowestKeyMapIterator(self.elements.iter_mut())
    }
}

pub struct LowestKeyMapIterator<'a>(IterMut<'a, Option<ChannelState>>);

impl<'a> Iterator for LowestKeyMapIterator<'a> {
    type Item = &'a mut ChannelState;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next() {
                None => return None,
                Some(None) => continue, // skip
                Some(Some(t)) => return Some(t),
            }
        }
    }
}
