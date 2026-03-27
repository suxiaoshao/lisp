(let loop ((n 5) (acc 1))
  (if (= n 0)
      acc
      (loop (- n 1) (* acc n))))
