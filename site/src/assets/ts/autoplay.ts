// A recording plays once, when half of it is on screen in a visible tab, and
// only when the reader accepts motion. With JavaScript blocked, the poster stays.
export function autoplay(video: HTMLVideoElement): void {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  let inView = false;
  let started = false;
  const start = () => {
    if (started || !inView || document.hidden) return;
    started = true;
    watch.disconnect();
    video.play().catch(() => {
      // The browser refused playback: the poster stays.
    });
  };
  const watch = new IntersectionObserver((entries) => {
    inView = entries.some((entry) => entry.isIntersecting);
    start();
  }, { threshold: 0.5 });
  watch.observe(video);
  document.addEventListener("visibilitychange", start);
}
