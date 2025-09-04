var video = document.getElementById('video');
var link = video.className;
console.log(link);
video.removeAttribute('class');

if(link.endsWith("m3u8") && Hls.isSupported()) {
  video.volume = 0.3;
  var hls = new Hls();
  hls.loadSource(link);
  hls.attachMedia(video);
  hls.on(Hls.Events.MANIFEST_PARSED,function() {
      video.play();
  });
} else {
	video.setAttribute("src", link);
	video.setAttribute("type", "video/mp4");
	video.addEventListener('canplay',function() {
		video.play();
	});
	video.volume = 0.3;
}
