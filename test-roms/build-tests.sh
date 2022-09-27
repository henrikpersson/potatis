#!/usr/bin/env bash

git submodule sync 6502_65C02_functional_tests
git submodule update 6502_65C02_functional_tests

SRC_DIR='./6502_65C02_functional_tests/ca65'
CONFIG="$SRC_DIR/example.cfg"
OUT_DIR='./bin'

FULL_TEST='6502_functional_test'
FULL_TEST_OUT='functional_test_full'
echo "$SRC_DIR/$FULL_TEST -> $OUT_DIR/$FULL_TEST_OUT"
ca65 -l "$OUT_DIR/$FULL_TEST_OUT.lst" "$SRC_DIR/$FULL_TEST.ca65" -o "$OUT_DIR/$FULL_TEST_OUT.o"
ld65 "$OUT_DIR/$FULL_TEST_OUT.o" -o "$OUT_DIR/$FULL_TEST_OUT.bin" -m "$OUT_DIR/$FULL_TEST_OUT.map" -C $CONFIG
echo "done"

NO_BCD_TEST_OUT='functional_test_bcd_disabled'
TMP='./tmp.ca65'
sed 's/disable_decimal = 0/disable_decimal = 1/' "$SRC_DIR/$FULL_TEST.ca65" > $TMP

echo "$SRC_DIR/$FULL_TEST (bcd disabled) -> $OUT_DIR/$NO_BCD_TEST_OUT"
ca65 -l "$OUT_DIR/$NO_BCD_TEST_OUT.lst" $TMP -o "$OUT_DIR/$NO_BCD_TEST_OUT.o"
ld65 "$OUT_DIR/$NO_BCD_TEST_OUT.o" -o "$OUT_DIR/$NO_BCD_TEST_OUT.bin" -m "$OUT_DIR/$NO_BCD_TEST_OUT.map" -C $CONFIG
echo "done"

EXTENDED_TEST='65C02_extended_opcodes_test'
EXTENDED_TEST_OUT='extended_test'
echo "$SRC_DIR/$EXTENDED_TEST -> $OUT_DIR/$EXTENDED_TEST_OUT"
ca65 -l "$OUT_DIR/$EXTENDED_TEST_OUT.lst" "$SRC_DIR/$EXTENDED_TEST.ca65" -o "$OUT_DIR/$EXTENDED_TEST_OUT.o"
ld65 "$OUT_DIR/$EXTENDED_TEST_OUT.o" -o "$OUT_DIR/$EXTENDED_TEST_OUT.bin" -m "$OUT_DIR/$EXTENDED_TEST_OUT.map" -C $CONFIG
echo "done"

rm $TMP