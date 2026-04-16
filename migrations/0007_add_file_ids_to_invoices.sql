ALTER TABLE invoices
    ADD COLUMN xml_file_id INT REFERENCES files(id),
    ADD COLUMN pdf_file_id INT REFERENCES files(id);
