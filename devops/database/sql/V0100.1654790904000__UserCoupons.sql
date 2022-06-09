CREATE TABLE stripe_user_coupons (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    coupon VARCHAR NOT NULL,
    PRIMARY KEY(user_id, coupon)
);