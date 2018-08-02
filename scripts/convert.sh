BINARY=../target/release/oscar-cli
MONO_LIST=mono_list.txt
STEREO_LIST=stereo_list.txt

for VAL in `cat $MONO_LIST`
do
$BINARY convert -w 10 -d -s 4 -f png $1$VAL $2$VAL;
done

for VAL in `cat $STEREO_LIST`
do
$BINARY convert_stereo -w 10 -d -s 4 -f png $1$VAL $2$VAL;
done
