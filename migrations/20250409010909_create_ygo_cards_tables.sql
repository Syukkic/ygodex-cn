-- Add migration script here
CREATE TABLE
    IF NOT EXISTS ygo_cards (
        cid INTEGER PRIMARY KEY,
        id BIGINT,
        cn_name TEXT,
        sc_name TEXT,
        md_name TEXT,
        nwbbs_n TEXT,
        cnocg_n TEXT,
        jp_ruby TEXT,
        jp_name TEXT,
        en_name TEXT,
        types TEXT,
        pdesc TEXT,
        "desc" TEXT,
        ot INTEGER,
        setcode BIGINT,
        "type" INTEGER,
        atk INTEGER,
        def INTEGER,
        level INTEGER,
        race INTEGER,
        attribute INTEGER,
        is_extra BOOLEAN
    );