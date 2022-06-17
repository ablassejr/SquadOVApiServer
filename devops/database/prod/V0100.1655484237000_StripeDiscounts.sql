-- 100%
INSERT INTO squadov.stripe_user_coupons (
    user_id,
    coupon
) VALUES 
    (191, 'TpTQhirK'),
    (391, 'TpTQhirK'),
    (31965, 'TpTQhirK');

-- 45%
INSERT INTO squadov.stripe_user_coupons(
    user_id,
    coupon
)
SELECT id, 'tJqTnbll'
FROM squadov.users
WHERE registration_time IS NULL
    OR registration_time < TO_TIMESTAMP(1625097600);

-- 30%
INSERT INTO squadov.stripe_user_coupons(
    user_id,
    coupon
)
SELECT id, 'd8fa2yBw'
FROM squadov.users
WHERE registration_time >= TO_TIMESTAMP(1625097600)
    AND registration_time < TO_TIMESTAMP(1640995200);

-- 15%
INSERT INTO squadov.stripe_user_coupons(
    user_id,
    coupon
)
SELECT id, 'FqJJhRTy'
FROM squadov.users
WHERE registration_time >= TO_TIMESTAMP(1640995200)
    AND registration_time < TO_TIMESTAMP(1655424000);