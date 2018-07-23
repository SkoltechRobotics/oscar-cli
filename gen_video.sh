DIR=$1$2
echo [`date`] "Processing: " $DIR
INPUT=$DIR%d.png;
OUT_HQ=$DIR"video_hq.webm"
OUT=$DIR"video.webm"

ffmpeg -i $INPUT -c:v libvpx-vp9 -pass 1 -b:v 2000k -maxrate 3000k -threads 8 -speed 4 \
  -tile-columns 6 -frame-parallel 1 -threads 1 \
  -r 30 -vf "histeq" -loglevel panic \
  -f webm -y /dev/null

ffmpeg -i $INPUT -c:v libvpx-vp9 -pass 2 -b:v 2000k -maxrate 3000k -threads 8 -speed 1 \
  -tile-columns 6 -frame-parallel 1 -threads 1 -auto-alt-ref 1 -lag-in-frames 25 \
  -r 30 -vf "histeq" -loglevel panic \
  -f webm -y $OUT_HQ &&

echo [`date`] "Finished histeq: " $DIR &&

ffmpeg -i $INPUT -c:v libvpx-vp9 -pass 1 -b:v 2000k -maxrate 3000k -threads 8 -speed 4 \
  -tile-columns 6 -frame-parallel 1 -threads 1 \
  -r 30 -loglevel panic \
  -f webm -y /dev/null

ffmpeg -i $INPUT -c:v libvpx-vp9 -pass 2 -b:v 2000k -maxrate 3000k -threads 8 -speed 1 \
  -tile-columns 6 -frame-parallel 1 -threads 1 -auto-alt-ref 1 -lag-in-frames 25 \
  -r 30 -loglevel panic \
  -f webm -y $OUT

echo [`date`] "Finished normal: " $DIR
