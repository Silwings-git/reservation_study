use std::collections::VecDeque;

use crate::{
    FilterPager, ReservationFilter,
    pager::{Id, PageInfo, Pager, Paginator},
};

impl ReservationFilter {
    pub fn get_pager<T: Id>(&self, data: &mut VecDeque<T>) -> FilterPager {
        let page_info = self.page_info();
        let pager = page_info.get_pager(data);
        pager.into()
    }

    fn page_info(&self) -> PageInfo {
        PageInfo {
            cursor: self.cursor,
            page_size: self.page_size,
            desc: self.desc,
        }
    }
}

impl From<Pager> for FilterPager {
    fn from(value: Pager) -> Self {
        Self {
            prev: value.prev,
            next: value.next,
            total: value.total,
        }
    }
}
