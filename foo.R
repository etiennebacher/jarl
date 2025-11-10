x <- any(is.na(mtcars)) # Should be anyNA()

f <- function(x) {
  apply(x, 1, mean) # Should be rowMeans()
}
