ALTER TYPE invoice_type_enum RENAME TO invoice_type_enum_old;
CREATE TYPE invoice_type_enum AS ENUM ('I', 'E', 'T', 'P', 'N');

ALTER TABLE invoices
    ALTER COLUMN invoice_type TYPE invoice_type_enum
    USING (CASE invoice_type::TEXT
        WHEN 'ingreso'  THEN 'I'
        WHEN 'egreso'   THEN 'E'
        WHEN 'traslado' THEN 'T'
        WHEN 'pago'     THEN 'P'
        WHEN 'nómina'   THEN 'N'
        ELSE 'I'
    END)::invoice_type_enum;

DROP TYPE invoice_type_enum_old;
