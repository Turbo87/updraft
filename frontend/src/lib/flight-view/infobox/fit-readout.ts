export function fitReadout(area: HTMLElement) {
  let readout = area.firstElementChild as HTMLElement;
  let step = 0;
  let active = true;

  function applySize() {
    readout.style.fontSize = `${40 * 0.9 ** step}px`;
  }

  function fit() {
    if (!active || !area.clientWidth || !area.clientHeight) return;
    let { width, height } = area.getBoundingClientRect();
    function fits(space = 1) {
      let bounds = readout.getBoundingClientRect();
      return bounds.width <= width * space && bounds.height <= height * space;
    }

    applySize();
    while (!fits() && step < 36) {
      step++;
      applySize();
    }
    // A larger step needs spare room before it replaces the current size.
    while (step > 0) {
      step--;
      applySize();
      if (!fits(0.9)) {
        step++;
        applySize();
        break;
      }
    }
  }

  let resize = new ResizeObserver(fit);
  resize.observe(area);
  let content = new MutationObserver(fit);
  content.observe(readout, { childList: true, characterData: true, subtree: true });
  document.fonts.addEventListener('loadingdone', fit);
  void document.fonts.ready.then(fit);
  fit();

  return () => {
    active = false;
    resize.disconnect();
    content.disconnect();
    document.fonts.removeEventListener('loadingdone', fit);
  };
}
