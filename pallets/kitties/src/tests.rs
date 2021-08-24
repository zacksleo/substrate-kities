use super::*;
use crate::mock::{new_test_ext, Event as TestEvent, Kitties, Origin, System, Test};
use crate::Error;
use frame_support::{assert_noop, assert_ok};

#[test]
fn create_max_count_overflow_test() {
	new_test_ext().execute_with(|| {
		KittiesCount::<Test>::put(u32::max_value());
		assert_noop!(Kitties::create(Origin::signed(1)), Error::<Test>::KittiesCountOverflow);
	});
}

#[test]
fn create_test() {
	new_test_ext().execute_with(|| {
		assert_ok!(Kitties::create(Origin::signed(1)));
		System::assert_has_event(TestEvent::Kitties(Event::KittyCreated(1, 1)));
	});
}

#[test]
fn transfer_test() {
	new_test_ext().execute_with(|| {
		assert_ok!(Kitties::create(Origin::signed(1)));
		assert_ok!(Kitties::transfer(Origin::signed(1), 2, 1));
		System::assert_has_event(TestEvent::Kitties(Event::KittyTransfered(1, 2, 1)));
	});
}

#[test]
fn transfer_not_owner_test() {
	new_test_ext().execute_with(|| {
		assert_ok!(Kitties::create(Origin::signed(1)));

		assert_noop!(Kitties::transfer(Origin::signed(2), 1, 1), Error::<Test>::NotOwnerOfKitty);
	});
}

#[test]
fn breed_test() {
	new_test_ext().execute_with(|| {
		let _ = Kitties::create(Origin::signed(1));
		let _ = Kitties::create(Origin::signed(1));

		assert_ok!(Kitties::breed(Origin::signed(1), 1, 2));
		System::assert_has_event(TestEvent::Kitties(Event::KittyCreated(1, 3)));
	});
}

#[test]
fn breed_fail_with_same_id() {
	new_test_ext().execute_with(|| {
		assert_noop!(Kitties::breed(Origin::signed(1), 1, 1), Error::<Test>::SameParentIndex);
	});
}

#[test]
fn breed_fail_with_invalid_index() {
	new_test_ext().execute_with(|| {
		assert_noop!(Kitties::breed(Origin::signed(1), 1, 2), Error::<Test>::InvalidKittyIndex);
	});
}

#[test]
fn breed_fail_with_invalid_owner() {
	new_test_ext().execute_with(|| {
		let _ = Kitties::create(Origin::signed(1));
		let _ = Kitties::create(Origin::signed(2));

		assert_noop!(Kitties::breed(Origin::signed(1), 1, 2), Error::<Test>::NotOwnerOfKitty);
	});
}

#[test]
fn breed_fail_with_count_overflow() {
	new_test_ext().execute_with(|| {
		let _ = Kitties::create(Origin::signed(1));
		let _ = Kitties::create(Origin::signed(1));

		KittiesCount::<Test>::put(u32::max_value());

		assert_noop!(Kitties::breed(Origin::signed(1), 1, 2), Error::<Test>::KittiesCountOverflow);
	});
}
