#!/bin/bash
cargo build

data=test.bin
day1=2021-03-29
day2=2021-03-30
day3=2021-03-31
day4=2021-04-01
day5=2021-04-02

config1=test_config.toml
config2=test_config_min_break.toml

output=output.txt
target=target.txt

if ! command -v faketime &> /dev/null
then
    echo "faketime is needed for this script to work."
	echo "you can get it from: https://github.com/wolfcw/libfaketime"
    exit
fi


tt() {
	current_date=$1
	shift
	current_time=$1
	shift
	config=$1
	shift
	faketime "$current_date $current_time" ../target/debug/tt -c $config -d $data "$@"
}

rm $data 2> /dev/null
rm $output 2> /dev/null
touch $output

tt $day1 "08:00" $config1 start
tt $day1 "12:00" $config1 stop pause
tt $day1 "12:15" $config1 start
tt $day1 "16:15" $config1 stop
echo "D1 - show -p:" >> $output
tt $day1 "17:15" $config1 show -p >> $output
echo "" >> $output
echo "D1 - show -p -r:" >> $output
tt $day1 "17:15" $config1 show -p -r >> $output
echo "" >> $output

echo "" >> $output
echo "================" >> $output
echo "" >> $output

tt $day2 "08:00" $config2 start
tt $day2 "12:00" $config2 stop pause
tt $day2 "12:15" $config2 start
tt $day2 "16:15" $config2 stop
echo "D2 - show -p:" >> $output
tt $day2 "17:15" $config2 show -p >> $output
echo "" >> $output
echo "D2 - show -p -r:" >> $output
tt $day2 "17:15" $config2 show -p -r >> $output
echo "" >> $output

echo "" >> $output
echo "================" >> $output
echo "" >> $output

tt $day3 "08:00" $config1 start
tt $day3 "12:00" $config1 stop pause
tt $day3 "12:15" $config1 start
tt $day3 "15:15" $config1 stop
echo "D3 - show -p week:" >> $output
tt $day3 "17:15" $config2 show -p week >> $output
echo "" >> $output

echo "" >> $output
echo "================" >> $output
echo "" >> $output

tt $day4 "08:00" $config1 start
tt $day4 "12:00" $config1 stop pause
tt $day4 "12:15" $config1 start
echo "D4 - show -r -p week:" >> $output
tt $day4 "16:00" $config1 show -r -p week >> $output
echo "" >> $output
echo "D4 - show -r -p:" >> $output
tt $day4 "16:00" $config1 show -r -p >> $output
echo "" >> $output
tt $day4 "16:15" $config1 stop

echo "" >> $output
echo "================" >> $output
echo "" >> $output

tt $day5 "08:00" $config1 start
tt $day5 "12:00" $config1 stop pause
tt $day5 "12:15" $config1 start
echo "D5 - show -r -p week:" >> $output
tt $day5 "16:00" $config1 show -r -p week >> $output
echo "" >> $output
echo "D5 - show -r -p:" >> $output
tt $day5 "16:00" $config1 show -r -p >> $output
echo "" >> $output
tt $day5 "16:15" $config1 stop

if ! diff -q $output $target &>/dev/null; then
	>&2 echo "Test output changed."
	diff -u --color $output $target
else
	echo "Test output is the same."
fi

rm $data 2> /dev/null
rm $output 2> /dev/null
