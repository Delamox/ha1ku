var video = document.getElementById('video');
if(Hls.isSupported()) {
    video.volume = 0.3;
    var hls = new Hls();
    hls.loadSource(video.className);
    video.removeAttribute('class');
    hls.attachMedia(video);
    hls.on(Hls.Events.MANIFEST_PARSED,function() {
        video.play();
    });
} else if (video.canPlayType('application/vnd.apple.mpegurl')) {
	video.src = url;
	video.addEventListener('canplay',function() {
		video.play();
	});
	video.volume = 0.3;
}
