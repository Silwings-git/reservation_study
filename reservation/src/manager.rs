use crate::{QueryBuilderExt, ReservationId, ReservationManager, Rsvp};
use abi::{Error, FilterPager, ReservationQuery, ReservationStatus, Validator};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::types::PgRange;
use sqlx::{QueryBuilder, Row};

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, Error> {
        // generate a insert sql for the reservation
        if rsvp.start.is_none() || rsvp.end.is_none() {
            return Err(Error::InvalidTime);
        }

        let timespan: PgRange<DateTime<Utc>> = rsvp.get_timespan();

        let id = sqlx::query(
            "INSERT INTO rsvp.reservations(user_id,resource_id,timespan,note,status) VALUES ($1,$2,$3,$4,$5::rsvp.reservation_status) RETURNING id",
        )
        .bind(rsvp.user_id.clone())
        .bind(rsvp.resource_id.clone())
        .bind(timespan)
        .bind(rsvp.note.clone())
        .bind(rsvp.status().to_string())
        .fetch_one(&self.pool)
        .await?.get(0);

        rsvp.id = id;

        Ok(rsvp)
    }

    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        id.validate()?;
        // if current status is pending, change it to confirmed, otherwise do nothing
        let rsvp = sqlx::query_as(
            "UPDATE rsvp.reservations SET status = 'confirmed' WHERE id = $1 AND status = 'pending' RETURNING *",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(rsvp)
    }

    async fn update_note(
        &self,
        id: ReservationId,
        note: String,
    ) -> Result<abi::Reservation, Error> {
        id.validate()?;
        let rsvp =
            sqlx::query_as("UPDATE rsvp.reservations SET note = $1 WHERE id = $2 RETURNING *")
                .bind(note)
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(rsvp)
    }

    async fn delete(&self, id: ReservationId) -> Result<(), Error> {
        id.validate()?;
        sqlx::query("DELETE FROM rsvp.reservations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, Error> {
        id.validate()?;
        let rsvp = sqlx::query_as("SELECT * FROM rsvp.reservations WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(rsvp)
    }

    async fn query(&self, query: ReservationQuery) -> Result<Vec<abi::Reservation>, Error> {
        let mut builder = QueryBuilder::new("SELECT * FROM rsvp.reservations WHERE true");
        let rsvps = builder
            .push_and_bind_if_with(!query.resource_id.is_empty(), " AND resource_id = ", || {
                &query.resource_id
            })
            .push_and_bind_if_with(!query.user_id.is_empty(), " AND user_id = ", || {
                &query.user_id
            })
            .push_and_bind_if_with(
                !matches!(query.status(), ReservationStatus::Unknown),
                " AND status = ",
                || query.status().to_string(),
            )
            .push("::rsvp.reservation_status")
            .push(format!(
                " AND tstzrange('{}','{}') @> timespan ",
                query.pg_start_time_string(),
                query.pg_end_time_string()
            ))
            .push(format!(
                " ORDER BY lower(timespan) {}",
                if query.desc { "DESC" } else { "ASC" }
            ))
            .build_query_as()
            .fetch_all(&self.pool)
            .await?;
        Ok(rsvps)
    }

    async fn filter(
        &self,
        filter: abi::ReservationFilter,
    ) -> Result<(FilterPager, Vec<abi::Reservation>), abi::Error> {
        let mut builder = QueryBuilder::new("SELECT * FROM rsvp.reservations WHERE true ");
        let query = builder
            .push_and_bind_if_with(
                !filter.resource_id.is_empty(),
                " AND resource_id = ",
                || &filter.resource_id,
            )
            .push_and_bind_if_with(!filter.user_id.is_empty(), " AND user_id = ", || {
                &filter.user_id
            })
            .push_and_bind_if_with(
                !matches!(filter.status(), ReservationStatus::Unknown),
                " AND status = ",
                || filter.status().to_string(),
            )
            .push("::rsvp.reservation_status")
            .push_and_bind_if_with(
                filter.cursor.is_some(),
                if filter.desc {
                    " AND id <= "
                } else {
                    " AND id >= "
                },
                || filter.cursor.unwrap(),
            )
            .push(if filter.desc {
                " ORDER BY id DESC "
            } else {
                " ORDER BY id ASC "
            })
            .push(" LIMIT ")
            .push_bind(filter.page_size + 1 + if filter.cursor.is_some() { 1 } else { 0 })
            .build_query_as::<abi::Reservation>();

        let rsvps = query.fetch_all(&self.pool).await?;
        let mut rsvps = rsvps.into_iter().collect();

        let pager = filter.get_pager(&mut rsvps);

        Ok((pager, rsvps.into_iter().collect()))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use abi::{
        Reservation, ReservationConflictInfo, ReservationFilterBuilder, ReservationQueryBuilder,
    };
    use chrono::FixedOffset;
    use sqlx::PgPool;
    use sqlx_db_tester::TestPg;

    #[tokio::test]
    async fn reserve_should_work_for_valid_window() {
        let tdb: TestPg = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, _manager) = make_silwings_reservation(pool).await;
        println!("rsvp: {rsvp:?}");
        assert!(rsvp.id != 0);
    }

    #[tokio::test]
    async fn reserve_conflict_reservation_should_reject() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let rsvp1 = Reservation::new_pending(
            "silwingsId",
            "ocean-view-room-713",
            "2025-12-25T22:40:00+0800".parse().unwrap(),
            "2025-12-28T12:00:00+0800".parse().unwrap(),
            "hello.",
        );
        let rsvp2 = Reservation::new_pending(
            "alicedId",
            "ocean-view-room-713",
            "2025-12-26T22:40:00+0800".parse().unwrap(),
            "2025-12-30T12:00:00+0800".parse().unwrap(),
            "hello.",
        );

        let manager = ReservationManager::new(pool.clone());
        let _rsvp1 = manager.reserve(rsvp1).await.unwrap();
        let err = manager.reserve(rsvp2).await.unwrap_err();
        let offset = FixedOffset::east_opt(8 * 3600).unwrap();
        if let Error::ConflictReservation(ReservationConflictInfo::Parsed(info)) = err {
            assert_eq!(info.new.rid, "ocean-view-room-713");
            assert_eq!(
                info.new.start.with_timezone(&offset).to_rfc3339(),
                "2025-12-26T22:40:00+08:00"
            );
            assert_eq!(
                info.new.end.with_timezone(&offset).to_rfc3339(),
                "2025-12-30T12:00:00+08:00"
            );
            assert_eq!(info.old.rid, "ocean-view-room-713");
            assert_eq!(
                info.old.start.with_timezone(&offset).to_rfc3339(),
                "2025-12-25T22:40:00+08:00"
            );
            assert_eq!(
                info.old.end.with_timezone(&offset).to_rfc3339(),
                "2025-12-28T12:00:00+08:00"
            );
        } else {
            assert!(false);
        }
    }

    #[tokio::test]
    async fn reserve_change_status_should_work() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;

        let rsvp = manager.change_status(rsvp.id).await.unwrap();
        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);
    }

    #[tokio::test]
    async fn reserve_change_status_not_pending_should_do_nothging() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;

        let rsvp = manager.change_status(rsvp.id).await.unwrap();

        // change status again should do nothing
        let ret = manager.change_status(rsvp.id).await.unwrap_err();
        assert_eq!(ret, abi::Error::NotFound);
    }

    #[tokio::test]
    async fn update_note_should_work() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;
        let rsvp = manager
            .update_note(rsvp.id, "hello world".into())
            .await
            .unwrap();
        assert_eq!(rsvp.note, "hello world");
    }

    #[tokio::test]
    async fn get_reservation_should_work() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;
        let rsvp1 = manager.get(rsvp.id).await.unwrap();
        assert_eq!(rsvp, rsvp1);
    }

    #[tokio::test]
    async fn delete_reservation_should_work() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;
        manager.delete(rsvp.id).await.unwrap();
        let rsvp1 = manager.get(rsvp.id).await.unwrap_err();
        assert_eq!(rsvp1, abi::Error::NotFound);
    }

    #[tokio::test]
    async fn query_reservation_should_work() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;
        let query = ReservationQueryBuilder::default()
            .resource_id(rsvp.resource_id.clone())
            .user_id(rsvp.user_id.clone())
            .status(rsvp.status)
            .start(rsvp.start.unwrap())
            .end(rsvp.end.unwrap())
            .desc(true)
            .build()
            .unwrap();
        println!("查询条件: {query:?}");
        let rsvps = manager.query(query).await.unwrap();
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvps[0], rsvp);
    }

    #[tokio::test]
    async fn filter_reservations_should_work() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let (rsvp, manager) = make_silwings_reservation(pool).await;
        let filter = ReservationFilterBuilder::default()
            .user_id(rsvp.user_id.clone())
            .status(rsvp.status)
            .page_size(10)
            .build()
            .unwrap();

        let (pager, rsvps) = manager.filter(filter).await.unwrap();
        assert_eq!(pager.prev, None);
        assert_eq!(pager.next, None);
        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvps[0], rsvp);
    }

    async fn make_silwings_reservation(pool: PgPool) -> (Reservation, ReservationManager) {
        make_reservation(
            pool,
            "silwingsId",
            "ocean-view-room-713",
            "2025-05-28T22:40:00+0800",
            "2025-06-28T12:00:00+0800",
            "I'll arrive at 3pm. Please help to upgrade to execuitive room if possible.",
        )
        .await
    }

    fn get_tdb() -> TestPg {
        TestPg::new::<PathBuf>(
            "postgres://postgres:root@localhost:5432/reservation".to_string(),
            "../migrations".into(),
        )
    }

    async fn make_reservation(
        pool: PgPool,
        uid: &str,
        rid: &str,
        start: &str,
        end: &str,
        note: &str,
    ) -> (Reservation, ReservationManager) {
        let manager = ReservationManager::new(pool.clone());
        let rsvp = abi::Reservation::new_pending(
            uid,
            rid,
            start.parse().unwrap(),
            end.parse().unwrap(),
            note,
        );

        (manager.reserve(rsvp).await.unwrap(), manager)
    }
}
