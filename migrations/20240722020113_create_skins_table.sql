CREATE TABLE skins (
    id BIGINT PRIMARY KEY,
    price BIGINT NOT NULL,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    ema DOUBLE PRECISION NOT NULL
);

CREATE OR REPLACE FUNCTION update_skin_price_ema(
    p_id BIGINT,
    p_price BIGINT,
    p_date DATE DEFAULT CURRENT_DATE,
    p_smoothing_factor DOUBLE PRECISION DEFAULT 0.1
) RETURNS VOID AS $$
DECLARE
    v_current_date DATE;
    v_current_ema DOUBLE PRECISION;
BEGIN
    -- Try to get the current date and EMA for the skin
    SELECT date, ema
    INTO v_current_date, v_current_ema
    FROM skins
    WHERE id = p_id;

    IF FOUND THEN
        -- Record exists, check if the date has changed
        IF p_date > v_current_date THEN
            -- Calculate the new EMA
            v_current_ema := p_price * p_smoothing_factor + v_current_ema * (1 - p_smoothing_factor);

            -- Update the row with the new price, date, and EMA
            UPDATE skins
            SET price = p_price,
                date = p_date,
                ema = v_current_ema
            WHERE id = p_id;
        END IF;
    ELSE
        -- Record doesn't exist, insert a new row
        INSERT INTO skins (id, price, date, ema)
        VALUES (p_id, p_price, p_date, p_price::DOUBLE PRECISION);
    END IF;
END;
$$ LANGUAGE plpgsql;
