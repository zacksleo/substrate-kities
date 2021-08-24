use super::*;
use crate::mock::{new_test_ext, Kitties, Origin, Test};
use crate::Error;
use frame_support::{assert_noop, assert_ok};

#[test]
fn create_max_count_overflow_test() {
	new_test_ext().execute_with(|| {
		KittiesCount::<Test>::put(u32::max_value());
		assert_noop!(
			Kitties::create(Origin::signed(1)),
			Error::<Test>::KittiesCountOverflow,
		);
	});
}
