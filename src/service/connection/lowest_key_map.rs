pub struct LowestKeyMap<T> {
    capacity: usize,
    elements: Vec<Option<T>>
}

impl <T> LowestKeyMap<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            elements: Vec::with_capacity(1),
        }
    }

    pub fn len(&self) -> usize {
        let mut len = 0;
        for e in &self.elements {
            if e.is_some() { len += 1};
        }
        len
    }

    pub fn get(&mut self, i: usize) -> Option<&T> {
        match self.elements.get(i) {
            None => None,
            Some(None) => None,
            Some(Some(e)) => Some(&e)
        }
    }

    pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        match self.elements.get_mut(i) {
            None => None,
            Some(None) => None,
            Some(Some(t)) => Some(t),
        }
    }

    pub fn insert(&mut self, t: T) -> Result<usize, T> {
        for i in 0 .. self.elements.len() {
            if self.elements[i].is_none() {
                self.elements[i] = Some(t);
                return Ok(i)
            }
        }
        if self.elements.len() < self.capacity {
            let i = self.elements.len();
            self.elements.push(Some(t));
            return Ok(i)
        }
        Err(t)
    }

    pub fn remove(&mut self, i: usize) {
        // TODO: shrink vector
        if i < self.elements.len() {
            self.elements[i] = None;
        }
    }
}
