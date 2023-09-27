UPDATE ais_vessels
SET
    call_sign = TRANSLATE(call_sign, '_- ', '');

UPDATE ais_vessels_historic
SET
    call_sign = TRANSLATE(call_sign, '_- ', '');
