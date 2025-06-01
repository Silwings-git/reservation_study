use crate::{ReservationId, ReservationManager, Rsvp};
use abi::{Error, ReservationQuery, Validator};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::types::PgRange;

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

    async fn query(&self, _query: ReservationQuery) -> Result<Vec<abi::Reservation>, Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use abi::{Reservation, ReservationConflictInfo};
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
            "postgres://silwings:root@localhost:5433/reservation".to_string(),
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
