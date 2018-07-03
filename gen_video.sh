cargo run --release -- convert_stereo -w 6 -d -s 2 -f pnm $1 $2 &&
mkdir $3 &&
cd $3 &&
ffmpeg -r 30 -i $2/%d.pnm -tune film -crf 22 -vf "histeq" video_hq.mp4 &&
ffmpeg -r 30 -i $2/%d.pnm -tune film -crf 22 video.mp4
#rm -r $2
