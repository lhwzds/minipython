left = tuple([1, 2]); right = tuple([1, 2])
emit('result', ordering=[1] < [2], reverse=[2] < [1], prefix=[1] < [1, 2],
     le=[1] <= [1], gt=[1, 1] > [1], ge=[1] >= [2], first_only=[1, 2] == [1, 3],
     tuple_equal=left == right)
