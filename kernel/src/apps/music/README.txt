if you want to add ur own music just do this:
ffmpeg -i song.mp3 -f u8 -acodec pcm_u8 -ac 1 -ar 11025 song.raw
