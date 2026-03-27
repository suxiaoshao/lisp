#lang racket
(let loop ((n 8) (acc 1))
  (if (= n 0)
      acc
      (loop (- n 1) (* acc n))))
