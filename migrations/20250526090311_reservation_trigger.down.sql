DROP TRIGGER reservations_trigger ON rsvp.reservations;
DROP FUNCTION rsvp.reservations_trigger();
DROP TABLE rsvp.reservation_changes CASCADE;
DROP TABLE rsvp.server_read_cursor CASCADE;
